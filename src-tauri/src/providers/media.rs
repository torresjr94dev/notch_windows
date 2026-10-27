//! Now Playing: la sesión de medios actual de Windows (GSMTC), la misma que alimenta el
//! panel de medios de la barra de tareas. Cubre Spotify, navegadores, el Reproductor
//! multimedia y cualquier app que se reporte a Windows.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use serde::Deserialize;
use tauri::State;
use windows::Foundation::TypedEventHandler;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session,
    GlobalSystemMediaTransportControlsSessionManager as SessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
};
use windows::Storage::Streams::DataReader;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

use crate::activity::{Activity, ActivityData, ActivityHub, NowPlaying, priority};

const ACTIVITY_ID: &str = "media";
/// Una canción en pausa deja de ocupar la isla después de este tiempo.
const PAUSED_TTL: Duration = Duration::from_secs(5 * 60);
/// Respaldo por si la app no avisa sus cambios (varias no emiten los de timeline).
const REFRESH_EVERY: Duration = Duration::from_secs(1);
/// Algunas apps (Spotify) cambian el título antes de tener lista la carátula.
const ARTWORK_RETRIES: u8 = 5;
/// Diferencia entre la época de Windows (1601) y la de Unix (1970), en unidades de 100 ns.
const WINDOWS_TO_UNIX_EPOCH: i64 = 116_444_736_000_000_000;

#[derive(Debug, Deserialize)]
#[serde(
    tag = "action",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum MediaCommand {
    TogglePlayPause,
    Next,
    Previous,
    Seek { position_ms: u64 },
}

enum Msg {
    SessionChanged,
    Refresh,
    Command(MediaCommand),
}

pub struct MediaState {
    tx: Sender<Msg>,
    /// Solo la carátula actual: (id, data URL).
    artwork: Arc<Mutex<Option<(String, String)>>>,
    /// El visualizador solo captura audio mientras esto es `true`.
    pub playing: Arc<AtomicBool>,
}

#[tauri::command]
pub fn media_control(command: MediaCommand, state: State<'_, MediaState>) {
    let _ = state.tx.send(Msg::Command(command));
}

#[tauri::command]
pub fn get_artwork(id: String, state: State<'_, MediaState>) -> Option<String> {
    let artwork = state.artwork.lock().unwrap_or_else(PoisonError::into_inner);
    artwork
        .as_ref()
        .filter(|(current, _)| *current == id)
        .map(|(_, url)| url.clone())
}

pub fn start(hub: ActivityHub) -> std::io::Result<MediaState> {
    let (tx, rx) = mpsc::channel();
    let state = MediaState {
        tx: tx.clone(),
        artwork: Arc::default(),
        playing: Arc::default(),
    };
    let mut worker = Worker {
        hub,
        artwork: state.artwork.clone(),
        playing: state.playing.clone(),
        published: None,
        artwork_key: None,
        artwork_attempts: 0,
    };
    thread::Builder::new()
        .name("media-provider".into())
        .spawn(move || {
            if let Err(err) = worker.run(tx, rx) {
                eprintln!("el provider de medios se detuvo: {err}");
            }
        })?;
    Ok(state)
}

/// Sesión con sus suscripciones; al soltarla se desuscribe.
struct Attached {
    session: Session,
    tokens: [i64; 3],
}

impl Drop for Attached {
    fn drop(&mut self) {
        let [media, playback, timeline] = self.tokens;
        let _ = self.session.RemoveMediaPropertiesChanged(media);
        let _ = self.session.RemovePlaybackInfoChanged(playback);
        let _ = self.session.RemoveTimelinePropertiesChanged(timeline);
    }
}

struct Worker {
    hub: ActivityHub,
    artwork: Arc<Mutex<Option<(String, String)>>>,
    playing: Arc<AtomicBool>,
    /// Lo último publicado. Sin este filtro, una canción en pausa que caducó volvería a
    /// aparecer al siguiente refresco.
    published: Option<NowPlaying>,
    artwork_key: Option<u64>,
    artwork_attempts: u8,
}

impl Worker {
    fn run(&mut self, tx: Sender<Msg>, rx: Receiver<Msg>) -> windows::core::Result<()> {
        // SAFETY: inicializa COM en este hilo; WinRT lo necesita para activar clases.
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok()? };
        let manager = SessionManager::RequestAsync()?.join()?;
        let notify = tx.clone();
        manager.CurrentSessionChanged(&TypedEventHandler::new(move |_, _| {
            let _ = notify.send(Msg::SessionChanged);
            Ok(())
        }))?;

        let mut attached = attach(&manager, &tx);
        loop {
            match rx.recv_timeout(REFRESH_EVERY) {
                Ok(Msg::SessionChanged) => attached = attach(&manager, &tx),
                Ok(Msg::Command(command)) => {
                    if let Some(a) = &attached {
                        let _ = execute(&a.session, command);
                    }
                }
                Ok(Msg::Refresh) | Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return Ok(()),
            }
            let snapshot = match attached.as_ref().map(|a| self.read(&a.session)) {
                None => None,
                Some(Ok(snapshot)) => snapshot,
                // Un error suelto de la app (p. ej. mientras cambia de canción) no debe
                // borrar la actividad: se reintenta en el siguiente refresco.
                Some(Err(_)) => continue,
            };
            self.publish(snapshot);
        }
    }

    fn read(&mut self, session: &Session) -> windows::core::Result<Option<NowPlaying>> {
        let status = session.GetPlaybackInfo()?.PlaybackStatus()?;
        if !matches!(status, Status::Playing | Status::Paused) {
            return Ok(None);
        }
        let props = session.TryGetMediaPropertiesAsync()?.join()?;
        let title = props.Title()?.to_string();
        if title.is_empty() {
            return Ok(None);
        }
        let artist = props.Artist()?.to_string();
        let album = props.AlbumTitle()?.to_string();
        let app = app_display_name(&session.SourceAppUserModelId()?.to_string());

        let mut key = DefaultHasher::new();
        (&title, &artist, &album, &app).hash(&mut key);
        let key = key.finish();
        if self.artwork_key != Some(key) {
            self.artwork_key = Some(key);
            self.artwork_attempts = 0;
            *self.artwork.lock().unwrap_or_else(PoisonError::into_inner) = None;
        }
        let has_artwork = self
            .artwork
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_some();
        if !has_artwork && self.artwork_attempts < ARTWORK_RETRIES {
            self.artwork_attempts += 1;
            if let Some(url) = props.Thumbnail().ok().and_then(|t| read_data_url(&t).ok()) {
                *self.artwork.lock().unwrap_or_else(PoisonError::into_inner) =
                    Some((format!("{key:x}"), url));
            }
        }
        let artwork_id = self
            .artwork
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .map(|(id, _)| id.clone());

        let timeline = session.GetTimelineProperties()?;
        let to_ms = |hns: i64| (hns.max(0) / 10_000) as u64;
        let duration_ms = to_ms(timeline.EndTime()?.Duration - timeline.StartTime()?.Duration);
        Ok(Some(NowPlaying {
            title,
            artist,
            album,
            app,
            artwork_id,
            playing: status == Status::Playing,
            position_ms: to_ms(timeline.Position()?.Duration),
            duration_ms,
            sampled_at_ms: sampled_at_ms(timeline.LastUpdatedTime()?.UniversalTime, now_ms()),
        }))
    }

    fn publish(&mut self, snapshot: Option<NowPlaying>) {
        self.playing.store(
            snapshot.as_ref().is_some_and(|s| s.playing),
            Ordering::Relaxed,
        );
        if snapshot == self.published {
            return;
        }
        match &snapshot {
            None => self.hub.remove(ACTIVITY_ID),
            Some(now_playing) => self.hub.upsert(Activity {
                id: ACTIVITY_ID.into(),
                priority: if now_playing.playing {
                    priority::MEDIA_PLAYING
                } else {
                    priority::MEDIA_PAUSED
                },
                transient: false,
                ttl: (!now_playing.playing).then_some(PAUSED_TTL),
                data: ActivityData::NowPlaying(now_playing.clone()),
            }),
        }
        self.published = snapshot;
    }
}

/// Se suscribe a la sesión actual. Cada evento solo pide un refresco: leer todo de nuevo
/// es barato y evita razonar sobre qué cambió.
fn attach(manager: &SessionManager, tx: &Sender<Msg>) -> Option<Attached> {
    let session = manager.GetCurrentSession().ok()?;
    let tokens = [
        session.MediaPropertiesChanged(&refresh_on(tx)).ok()?,
        session.PlaybackInfoChanged(&refresh_on(tx)).ok()?,
        session.TimelinePropertiesChanged(&refresh_on(tx)).ok()?,
    ];
    let _ = tx.send(Msg::Refresh);
    Some(Attached { session, tokens })
}

/// Handler que solo pide un refresco. Genérico porque cada evento de la sesión tiene su
/// propio tipo de argumentos.
fn refresh_on<Args: windows::core::RuntimeType + 'static>(
    tx: &Sender<Msg>,
) -> TypedEventHandler<Session, Args> {
    let tx = tx.clone();
    TypedEventHandler::new(move |_, _| {
        let _ = tx.send(Msg::Refresh);
        Ok(())
    })
}

fn execute(session: &Session, command: MediaCommand) -> windows::core::Result<bool> {
    match command {
        MediaCommand::TogglePlayPause => session.TryTogglePlayPauseAsync()?.join(),
        MediaCommand::Next => session.TrySkipNextAsync()?.join(),
        MediaCommand::Previous => session.TrySkipPreviousAsync()?.join(),
        MediaCommand::Seek { position_ms } => session
            .TryChangePlaybackPositionAsync(position_ms as i64 * 10_000)?
            .join(),
    }
}

fn read_data_url(
    thumbnail: &windows::Storage::Streams::IRandomAccessStreamReference,
) -> windows::core::Result<String> {
    let stream = thumbnail.OpenReadAsync()?.join()?;
    let size = stream.Size()? as u32;
    if size == 0 {
        return Err(windows::core::Error::empty());
    }
    let reader = DataReader::CreateDataReader(&stream.GetInputStreamAt(0)?)?;
    reader.LoadAsync(size)?.join()?;
    let mut bytes = vec![0u8; size as usize];
    reader.ReadBytes(&mut bytes)?;
    let mime = stream.ContentType()?.to_string();
    let mime = if mime.is_empty() {
        "image/jpeg".into()
    } else {
        mime
    };
    Ok(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or_default()
}

/// Cuándo se midió la posición, en ms Unix. Varias apps nunca llenan `LastUpdatedTime`
/// (o lo mandan en el futuro); en ese caso la medición es "ahora".
fn sampled_at_ms(last_updated: i64, now_ms: u64) -> u64 {
    let unix_ms = (last_updated - WINDOWS_TO_UNIX_EPOCH) / 10_000;
    if last_updated <= 0 || unix_ms <= 0 || unix_ms as u64 > now_ms {
        now_ms
    } else {
        unix_ms as u64
    }
}

/// Nombre legible a partir del AppUserModelID de la sesión.
fn app_display_name(aumid: &str) -> String {
    const KNOWN: [(&str, &str); 13] = [
        ("spotify", "Spotify"),
        ("msedge", "Edge"),
        ("chrome", "Chrome"),
        // Firefox se registra con un hash fijo en vez de su nombre.
        ("308046b0af4a39cb", "Firefox"),
        ("firefox", "Firefox"),
        ("brave", "Brave"),
        ("opera", "Opera"),
        ("zunemusic", "Reproductor multimedia"),
        ("applemusic", "Apple Music"),
        ("tidal", "TIDAL"),
        ("deezer", "Deezer"),
        ("vlc", "VLC"),
        ("youtube", "YouTube Music"),
    ];
    let lower = aumid.to_lowercase();
    if let Some((_, name)) = KNOWN.iter().find(|(needle, _)| lower.contains(needle)) {
        return (*name).to_owned();
    }
    // "Empresa.App_hash!App", "C:\ruta\app.exe" o "App.exe" → "App".
    let tail = aumid.rsplit(['!', '\\', '/']).next().unwrap_or(aumid);
    let tail = tail
        .strip_suffix(".exe")
        .or_else(|| tail.strip_suffix(".EXE"))
        .unwrap_or(tail);
    tail.rsplit('.').next().unwrap_or(tail).to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_names_are_human() {
        assert_eq!(app_display_name("Spotify.exe"), "Spotify");
        assert_eq!(
            app_display_name("SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify"),
            "Spotify"
        );
        assert_eq!(app_display_name("MSEdge"), "Edge");
        assert_eq!(app_display_name("308046B0AF4A39CB"), "Firefox");
        assert_eq!(
            app_display_name("Microsoft.ZuneMusic_8wekyb3d8bbwe!Microsoft.ZuneMusic"),
            "Reproductor multimedia"
        );
        assert_eq!(app_display_name(r"C:\Apps\Foobar2000.exe"), "Foobar2000");
        assert_eq!(app_display_name("Contoso.Player_abc!App"), "App");
    }

    #[test]
    fn sampled_at_converts_windows_time() {
        // 2026-01-01T00:00:00Z
        let unix_ms = 1_767_225_600_000u64;
        let windows_time = unix_ms as i64 * 10_000 + WINDOWS_TO_UNIX_EPOCH;
        assert_eq!(sampled_at_ms(windows_time, unix_ms + 5_000), unix_ms);
    }

    #[test]
    fn sampled_at_falls_back_to_now_for_bogus_times() {
        let now = 1_767_225_600_000u64;
        assert_eq!(sampled_at_ms(0, now), now);
        let future = (now as i64 + 60_000) * 10_000 + WINDOWS_TO_UNIX_EPOCH;
        assert_eq!(sampled_at_ms(future, now), now);
    }

    #[test]
    fn media_commands_parse_from_frontend_json() {
        let seek: MediaCommand =
            serde_json::from_str(r#"{"action":"seek","positionMs":1500}"#).unwrap();
        assert!(matches!(seek, MediaCommand::Seek { position_ms: 1500 }));
        let next: MediaCommand = serde_json::from_str(r#"{"action":"next"}"#).unwrap();
        assert!(matches!(next, MediaCommand::Next));
    }
}

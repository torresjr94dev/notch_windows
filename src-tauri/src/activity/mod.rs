//! Actividades: lo que la isla muestra. Los *providers* (media, volumen, API local…)
//! publican actividades en el [`ActivityHub`]; un hilo árbitro decide cuál va en la
//! pastilla y cuál en la burbuja del split, y emite la vista a todas las islas.
//!
//! El frontend solo dibuja la vista: los tipos de aquí tienen su espejo en
//! `src/lib/activity.svelte.ts` y deben mantenerse en sync.

mod board;

use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use board::Board;
pub use board::IslandView;

pub const VIEW_EVENT: &str = "island:view";

/// Prioridades: la actividad con el número más alto se queda con la pastilla.
pub mod priority {
    /// Avisos de un par de segundos: volumen, brillo, Bloq Mayús, cargador.
    pub const TRANSIENT: u8 = 100;
    pub const MEDIA_PLAYING: u8 = 50;
    pub const MEDIA_PAUSED: u8 = 10;
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NowPlaying {
    pub title: String,
    pub artist: String,
    pub album: String,
    /// Nombre legible de la app que reproduce ("Spotify", "Chrome").
    pub app: String,
    /// Id de la carátula; los bytes se piden aparte con `get_artwork` para no
    /// reenviarlos en cada cambio de la vista.
    pub artwork_id: Option<String>,
    pub playing: bool,
    pub position_ms: u64,
    pub duration_ms: u64,
    /// Momento (ms desde epoch) en que se midió `position_ms`. El frontend interpola
    /// la posición actual en vez de que Rust emita un evento por segundo.
    pub sampled_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ActivityData {
    NowPlaying(NowPlaying),
    /// `level` de 0 a 1.
    Volume {
        level: f32,
        muted: bool,
    },
    /// `level` de 0 a 1.
    Brightness {
        level: f32,
    },
    Battery {
        percent: u8,
        charging: bool,
        /// Por debajo del umbral de batería baja y sin cargador.
        low: bool,
    },
    CapsLock {
        on: bool,
    },
}

/// Apps usando el micrófono o la cámara. No es una actividad: se muestra junto a
/// cualquiera, como los puntos de privacidad del iPhone.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Privacy {
    pub microphone: Vec<String>,
    pub camera: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    /// Una actividad por id: publicar el mismo id la reemplaza.
    pub id: String,
    pub priority: u8,
    /// Los transitorios nunca van a la burbuja del split ni se expanden.
    pub transient: bool,
    pub data: ActivityData,
    /// Cuánto vive desde su último cambio de contenido; `None` = hasta que la quiten.
    #[serde(skip)]
    pub ttl: Option<Duration>,
}

impl Activity {
    /// Aviso de sistema que se muestra `ttl` y desaparece solo.
    pub fn transient(id: &str, data: ActivityData, ttl: Duration) -> Self {
        Self {
            id: id.to_owned(),
            priority: priority::TRANSIENT,
            transient: true,
            data,
            ttl: Some(ttl),
        }
    }
}

pub enum Update {
    Upsert(Activity),
    Remove(String),
    Privacy(Privacy),
}

/// Punto de entrada para los providers. Es barato de clonar y se puede usar desde
/// cualquier hilo.
#[derive(Clone)]
pub struct ActivityHub {
    tx: Sender<Update>,
}

impl ActivityHub {
    pub fn upsert(&self, activity: Activity) {
        let _ = self.tx.send(Update::Upsert(activity));
    }

    pub fn remove(&self, id: impl Into<String>) {
        let _ = self.tx.send(Update::Remove(id.into()));
    }

    pub fn set_privacy(&self, privacy: Privacy) {
        let _ = self.tx.send(Update::Privacy(privacy));
    }
}

/// Última vista emitida, para las islas que nacen después (un monitor nuevo) y no
/// alcanzaron a recibir el evento.
#[derive(Default)]
pub struct CurrentView(Mutex<IslandView>);

#[tauri::command]
pub fn get_view(current: State<'_, CurrentView>) -> IslandView {
    current
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

/// Arranca el hilo árbitro. Duerme hasta que llega una actualización o vence el
/// siguiente transitorio, así que en reposo no consume nada.
pub fn start(app: &AppHandle) -> std::io::Result<ActivityHub> {
    let (tx, rx) = mpsc::channel::<Update>();
    app.manage(CurrentView::default());
    let app = app.clone();
    thread::Builder::new()
        .name("activity-arbiter".into())
        .spawn(move || {
            let mut board = Board::default();
            let mut last = IslandView::default();
            loop {
                let received = match board.next_expiry() {
                    Some(at) => rx.recv_timeout(at.saturating_duration_since(Instant::now())),
                    None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
                };
                match received {
                    Ok(update) => board.apply(update, Instant::now()),
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => break,
                }
                board.expire(Instant::now());

                let view = board.view();
                if view != last {
                    *app.state::<CurrentView>()
                        .0
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner) = view.clone();
                    let _ = app.emit(VIEW_EVENT, &view);
                    last = view;
                }
            }
        })?;
    Ok(ActivityHub { tx })
}

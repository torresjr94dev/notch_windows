//! Volumen: escucha el volumen maestro de la salida por defecto con un callback de Core
//! Audio (sin polling) y lo muestra como aviso transitorio. También lo ajusta cuando el
//! usuario gira la rueda del mouse sobre la isla.
//!
//! El OSD nativo de Windows sigue apareciendo: ocultarlo no tiene API oficial y los
//! hacks que existen se rompen entre builds de Windows (ver ADR-0005).

use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration;

use tauri::State;
use windows::Win32::Media::Audio::AUDIO_VOLUME_NOTIFICATION_DATA;
use windows::Win32::Media::Audio::Endpoints::{
    IAudioEndpointVolume, IAudioEndpointVolumeCallback, IAudioEndpointVolumeCallback_Impl,
};
use windows::Win32::System::Com::{CLSCTX_ALL, COINIT_MULTITHREADED, CoInitializeEx};
use windows::core::implement;

use super::audio;
use crate::activity::{Activity, ActivityData, ActivityHub};

const ACTIVITY_ID: &str = "system:volume";
const TTL: Duration = Duration::from_millis(1800);
/// Cada cuánto se revisa si cambió la salida por defecto, para re-suscribirse a la nueva.
const DEVICE_CHECK: Duration = Duration::from_secs(2);
/// El ajuste más grande que acepta un solo comando: un gesto no puede subir al 100% de golpe.
const MAX_STEP: f32 = 0.1;

enum Msg {
    /// Core Audio avisó un cambio: (nivel, silenciado).
    Changed(f32, bool),
    /// El frontend pidió ajustar el volumen (rueda del mouse).
    Adjust(f32),
}

#[implement(IAudioEndpointVolumeCallback)]
struct Listener {
    tx: Sender<Msg>,
}

impl IAudioEndpointVolumeCallback_Impl for Listener_Impl {
    fn OnNotify(&self, data: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> windows::core::Result<()> {
        // SAFETY: Core Audio pasa un puntero válido durante la llamada; se copia y se suelta.
        if let Some(data) = unsafe { data.as_ref() } {
            let _ = self
                .tx
                .send(Msg::Changed(data.fMasterVolume, data.bMuted.as_bool()));
        }
        Ok(())
    }
}

/// Canal hacia el hilo del provider, que es el dueño del endpoint de Core Audio.
pub struct VolumeControl(Sender<Msg>);

/// Sube o baja el volumen maestro `delta` (de -1 a 1, acotado a ±`MAX_STEP`).
#[tauri::command]
pub fn adjust_volume(delta: f32, control: State<'_, VolumeControl>) {
    if delta.is_finite() {
        let _ = control
            .0
            .send(Msg::Adjust(delta.clamp(-MAX_STEP, MAX_STEP)));
    }
}

pub fn spawn(hub: ActivityHub) -> std::io::Result<VolumeControl> {
    let (tx, rx) = mpsc::channel();
    let control = VolumeControl(tx.clone());
    thread::Builder::new()
        .name("volume-provider".into())
        .spawn(move || {
            if let Err(err) = run(&hub, tx, &rx) {
                eprintln!("el provider de volumen se detuvo: {err}");
            }
        })?;
    Ok(control)
}

fn run(hub: &ActivityHub, tx: Sender<Msg>, rx: &mpsc::Receiver<Msg>) -> windows::core::Result<()> {
    // SAFETY: inicializa COM en este hilo, requisito de Core Audio.
    unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok()? };
    let enumerator = audio::device_enumerator()?;
    let callback: IAudioEndpointVolumeCallback = Listener { tx }.into();

    loop {
        let (device, device_id) = audio::default_output(&enumerator)?;
        // SAFETY: llamadas COM; el callback vive en `callback` mientras esté registrado.
        let endpoint: IAudioEndpointVolume = unsafe { device.Activate(CLSCTX_ALL, None)? };
        unsafe { endpoint.RegisterControlChangeNotify(&callback)? };

        // Core Audio también avisa cambios que no son del volumen maestro (balance por
        // canal); solo se muestra el aviso si cambió lo que se dibuja.
        let mut last = None;
        loop {
            match rx.recv_timeout(DEVICE_CHECK) {
                Ok(Msg::Changed(level, muted)) if last != Some((level, muted)) => {
                    last = Some((level, muted));
                    hub.upsert(Activity::transient(
                        ACTIVITY_ID,
                        ActivityData::Volume { level, muted },
                        TTL,
                    ));
                }
                Ok(Msg::Changed(..)) => {}
                Ok(Msg::Adjust(delta)) => {
                    // SAFETY: llamadas COM sobre el endpoint vivo; el GUID de contexto es
                    // opcional y no se usa. El cambio dispara `OnNotify` y con eso el aviso.
                    unsafe {
                        let current = endpoint.GetMasterVolumeLevelScalar()?;
                        endpoint.SetMasterVolumeLevelScalar(
                            adjusted(current, delta),
                            std::ptr::null(),
                        )?;
                    }
                }
                Err(RecvTimeoutError::Timeout) => {
                    if audio::default_output(&enumerator)?.1 != device_id {
                        break;
                    }
                }
                Err(RecvTimeoutError::Disconnected) => return Ok(()),
            }
        }
        // SAFETY: mismo callback que se registró arriba.
        unsafe { endpoint.UnregisterControlChangeNotify(&callback)? };
    }
}

/// Nuevo nivel tras un ajuste, acotado a 0–1 y redondeado a pasos de 1% para que la
/// rueda no deje el volumen en 49.999%.
fn adjusted(level: f32, delta: f32) -> f32 {
    ((level + delta).clamp(0.0, 1.0) * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::adjusted;

    #[test]
    fn adjustments_are_clamped_and_rounded() {
        assert_eq!(adjusted(0.5, 0.02), 0.52);
        assert_eq!(adjusted(0.99, 0.05), 1.0);
        assert_eq!(adjusted(0.01, -0.05), 0.0);
        assert_eq!(adjusted(0.4999, 0.0), 0.5);
    }
}

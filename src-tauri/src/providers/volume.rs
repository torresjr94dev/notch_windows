//! Volumen: escucha el volumen maestro de la salida por defecto con un callback de Core
//! Audio (sin polling) y lo muestra como aviso transitorio.
//!
//! El OSD nativo de Windows sigue apareciendo: ocultarlo no tiene API oficial y los
//! hacks que existen se rompen entre builds de Windows (ver ADR-0005).

use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration;

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

#[implement(IAudioEndpointVolumeCallback)]
struct Listener {
    tx: Sender<(f32, bool)>,
}

impl IAudioEndpointVolumeCallback_Impl for Listener_Impl {
    fn OnNotify(&self, data: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> windows::core::Result<()> {
        // SAFETY: Core Audio pasa un puntero válido durante la llamada; se copia y se suelta.
        if let Some(data) = unsafe { data.as_ref() } {
            let _ = self.tx.send((data.fMasterVolume, data.bMuted.as_bool()));
        }
        Ok(())
    }
}

pub fn spawn(hub: ActivityHub) -> std::io::Result<()> {
    thread::Builder::new()
        .name("volume-provider".into())
        .spawn(move || {
            if let Err(err) = run(&hub) {
                eprintln!("el provider de volumen se detuvo: {err}");
            }
        })?;
    Ok(())
}

fn run(hub: &ActivityHub) -> windows::core::Result<()> {
    // SAFETY: inicializa COM en este hilo, requisito de Core Audio.
    unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok()? };
    let enumerator = audio::device_enumerator()?;
    let (tx, rx) = mpsc::channel();
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
                Ok(state) if last != Some(state) => {
                    last = Some(state);
                    let (level, muted) = state;
                    hub.upsert(Activity::transient(
                        ACTIVITY_ID,
                        ActivityData::Volume { level, muted },
                        TTL,
                    ));
                }
                Ok(_) => {}
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

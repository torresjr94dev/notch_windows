//! Helpers de Core Audio compartidos por el volumen y el visualizador.

use windows::Win32::Media::Audio::{
    IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator, eConsole, eRender,
};
use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance, CoTaskMemFree};

pub fn device_enumerator() -> windows::core::Result<IMMDeviceEnumerator> {
    // SAFETY: requiere COM inicializado en el hilo; lo hace cada provider al arrancar.
    unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }
}

/// La salida de audio por defecto (bocinas, audífonos…) y su id, para detectar cuando
/// el usuario cambia de salida.
pub fn default_output(
    enumerator: &IMMDeviceEnumerator,
) -> windows::core::Result<(IMMDevice, String)> {
    // SAFETY: llamada COM sin punteros de por medio.
    let device = unsafe { enumerator.GetDefaultAudioEndpoint(eRender, eConsole)? };
    let id = device_id(&device)?;
    Ok((device, id))
}

fn device_id(device: &IMMDevice) -> windows::core::Result<String> {
    // SAFETY: `GetId` devuelve un PWSTR que el llamador debe liberar con CoTaskMemFree.
    unsafe {
        let raw = device.GetId()?;
        let id = raw.to_string().unwrap_or_default();
        CoTaskMemFree(Some(raw.0 as *const _));
        Ok(id)
    }
}

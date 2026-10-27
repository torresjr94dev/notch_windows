//! "Iniciar con Windows": un valor en la clave `Run` del usuario (sin permisos de
//! administrador ni tareas programadas).
//!
//! Si el usuario lo apaga desde el Administrador de tareas, Windows no borra el valor de
//! `Run`: marca un flag en `StartupApproved\Run`. Por eso el estado real sale de las dos
//! claves, y al activarlo desde la bandeja se limpia ese flag.

use std::ffi::c_void;

use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_BINARY, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegGetValueW,
    RegSetKeyValueW,
};
use windows::core::HSTRING;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const APPROVED_KEY: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
const VALUE_NAME: &str = "NotchWindows";

pub fn is_enabled() -> bool {
    has_run_value() && !disabled_in_task_manager()
}

/// Activa o desactiva el arranque con Windows para el ejecutable actual.
pub fn set_enabled(enabled: bool) -> std::io::Result<()> {
    let (key, name) = (HSTRING::from(RUN_KEY), HSTRING::from(VALUE_NAME));
    if !enabled {
        // SAFETY: cadenas válidas; borrar un valor inexistente solo devuelve un error.
        let _ = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, &key, &name) };
        return Ok(());
    }
    // Entre comillas: la ruta puede tener espacios ("C:\Program Files\…").
    let command = format!("\"{}\"", std::env::current_exe()?.display());
    let wide: Vec<u16> = command.encode_utf16().chain([0]).collect();
    // SAFETY: `wide` vive durante la llamada y su tamaño en bytes incluye el nulo final.
    let result = unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            &key,
            &name,
            REG_SZ.0,
            Some(wide.as_ptr() as *const c_void),
            (wide.len() * size_of::<u16>()) as u32,
        )
    };
    result.ok().map_err(std::io::Error::other)?;
    // Si lo habían apagado desde el Administrador de tareas, el flag ganaría.
    // SAFETY: igual que arriba.
    let _ = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, &HSTRING::from(APPROVED_KEY), &name) };
    Ok(())
}

fn has_run_value() -> bool {
    let mut size = 0u32;
    // SAFETY: solo se pide el tamaño (sin búfer).
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            &HSTRING::from(RUN_KEY),
            &HSTRING::from(VALUE_NAME),
            RRF_RT_REG_SZ,
            None,
            None,
            Some(&mut size),
        )
    };
    result == ERROR_SUCCESS
}

fn disabled_in_task_manager() -> bool {
    let mut data = [0u8; 12];
    let mut size = data.len() as u32;
    // SAFETY: `data` tiene `size` bytes.
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            &HSTRING::from(APPROVED_KEY),
            &HSTRING::from(VALUE_NAME),
            RRF_RT_REG_BINARY,
            None,
            Some(data.as_mut_ptr() as *mut c_void),
            Some(&mut size),
        )
    };
    result == ERROR_SUCCESS && approved_flag_disables(&data[..size as usize])
}

/// El primer byte del valor de `StartupApproved` es un flag: par = habilitado
/// (`02`, `06`), impar = deshabilitado por el usuario (`03`, `07`).
fn approved_flag_disables(data: &[u8]) -> bool {
    data.first().is_some_and(|flag| flag & 1 == 1)
}

#[cfg(test)]
mod tests {
    use super::approved_flag_disables;

    #[test]
    fn task_manager_flags() {
        assert!(!approved_flag_disables(&[0x02, 0, 0, 0]));
        assert!(!approved_flag_disables(&[0x06, 0, 0, 0]));
        assert!(approved_flag_disables(&[0x03, 0, 0, 0, 0x8a, 0x1f]));
        assert!(approved_flag_disables(&[0x07]));
        assert!(!approved_flag_disables(&[]));
    }
}

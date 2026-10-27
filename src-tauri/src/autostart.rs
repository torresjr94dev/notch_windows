//! "Iniciar con Windows": un valor en la clave `Run` del usuario (sin permisos de
//! administrador ni tareas programadas).
//!
//! Si el usuario lo apaga desde el Administrador de tareas, Windows no borra el valor de
//! `Run`: marca un flag en `StartupApproved\Run`. Por eso el estado real sale de las dos
//! claves, y al activarlo desde la bandeja se limpia ese flag.
//!
//! Solo cuenta como activo si el valor apunta a **este** ejecutable: el que dejó una
//! copia de desarrollo (o una instalación anterior en otra ruta) arrancaría otro `.exe`.
//! Así la bandeja lo muestra apagado y activarlo lo re-apunta a esta copia.

use std::ffi::c_void;
use std::path::Path;

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
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    run_command().is_some_and(|command| launches(&command, &exe)) && !disabled_in_task_manager()
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

/// El comando registrado en `Run`, si existe.
fn run_command() -> Option<String> {
    let (key, name) = (HSTRING::from(RUN_KEY), HSTRING::from(VALUE_NAME));
    let mut size = 0u32;
    // SAFETY: primero solo se pide el tamaño (sin búfer).
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            &key,
            &name,
            RRF_RT_REG_SZ,
            None,
            None,
            Some(&mut size),
        )
    };
    if result != ERROR_SUCCESS {
        return None;
    }
    let mut buffer = vec![0u16; (size as usize).div_ceil(size_of::<u16>())];
    // SAFETY: `buffer` tiene al menos `size` bytes. Si el valor creció entre las dos
    // llamadas, devuelve ERROR_MORE_DATA y no se escribe fuera del búfer.
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            &key,
            &name,
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr() as *mut c_void),
            Some(&mut size),
        )
    };
    if result != ERROR_SUCCESS {
        return None;
    }
    let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    Some(String::from_utf16_lossy(&buffer[..len]))
}

/// ¿El comando de `Run` abre este ejecutable? Se escribe entre comillas, pero también
/// acepta el valor sin ellas; Windows no distingue mayúsculas en las rutas.
fn launches(command: &str, exe: &Path) -> bool {
    command
        .trim()
        .trim_matches('"')
        .eq_ignore_ascii_case(&exe.to_string_lossy())
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
    use std::path::Path;

    use super::{approved_flag_disables, launches};

    #[test]
    fn run_command_must_point_to_this_exe() {
        let installed = Path::new(r"C:\Users\Ana\AppData\Local\Notch Windows\notch-windows.exe");
        assert!(launches(
            r#""C:\Users\Ana\AppData\Local\Notch Windows\notch-windows.exe""#,
            installed
        ));
        assert!(launches(
            r"c:\users\ana\appdata\local\notch windows\NOTCH-WINDOWS.EXE",
            installed
        ));
        // El que dejó `tauri dev` no arranca la versión instalada.
        assert!(!launches(
            r#""C:\src\notch_windows\src-tauri\target\debug\notch-windows.exe""#,
            installed
        ));
        assert!(!launches("", installed));
    }

    #[test]
    fn task_manager_flags() {
        assert!(!approved_flag_disables(&[0x02, 0, 0, 0]));
        assert!(!approved_flag_disables(&[0x06, 0, 0, 0]));
        assert!(approved_flag_disables(&[0x03, 0, 0, 0, 0x8a, 0x1f]));
        assert!(approved_flag_disables(&[0x07]));
        assert!(!approved_flag_disables(&[]));
    }
}

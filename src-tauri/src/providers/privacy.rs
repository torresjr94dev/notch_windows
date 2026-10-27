//! Privacidad: qué apps usan el micrófono o la cámara. Se lee de la misma fuente que el
//! ícono de privacidad de la barra de tareas, el registro de `CapabilityAccessManager`:
//! una app está usando el dispositivo mientras su `LastUsedTimeStop` vale 0.
//!
//! Sin polling: `RegNotifyChangeKeyValue` despierta al hilo cuando Windows escribe ahí.

use std::thread;
use std::time::Duration;

use windows::Win32::Foundation::{
    CloseHandle, ERROR_NO_MORE_ITEMS, ERROR_SUCCESS, HANDLE, WIN32_ERROR,
};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_NOTIFY, KEY_READ, REG_NOTIFY_CHANGE_LAST_SET,
    REG_NOTIFY_CHANGE_NAME, RRF_RT_REG_QWORD, RegCloseKey, RegEnumKeyExW, RegGetValueW,
    RegNotifyChangeKeyValue, RegOpenKeyExW,
};
use windows::Win32::System::Threading::{CreateEventW, INFINITE, WaitForMultipleObjects};
use windows::core::{HSTRING, PWSTR};

use crate::activity::{ActivityHub, Privacy};

const CONSENT_STORE: &str =
    r"Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore";
/// Las apps de escritorio (no empaquetadas) viven en esta subclave, una por ejecutable.
const NON_PACKAGED: &str = "NonPackaged";
/// Windows escribe inicio y fin de uso en pasos separados: esperar un poco evita leer
/// el estado a medias.
const SETTLE: Duration = Duration::from_millis(100);

pub fn spawn(hub: ActivityHub) -> std::io::Result<()> {
    thread::Builder::new()
        .name("privacy-provider".into())
        .spawn(move || {
            if let Err(err) = run(&hub) {
                eprintln!("el provider de privacidad se detuvo: {err}");
            }
        })?;
    Ok(())
}

/// Una clave del registro vigilada con su evento de notificación.
struct Watched {
    key: HKEY,
    event: HANDLE,
}

impl Watched {
    fn open(capability: &str) -> windows::core::Result<Self> {
        let mut key = HKEY::default();
        let path = HSTRING::from(format!(r"{CONSENT_STORE}\{capability}"));
        // SAFETY: out-param válido; la clave se cierra en `Drop`.
        unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                &path,
                None,
                KEY_READ | KEY_NOTIFY,
                &mut key,
            )
            .ok()?;
        }
        // SAFETY: evento sin nombre ni atributos; se cierra en `Drop`.
        let event = unsafe { CreateEventW(None, false, false, None)? };
        Ok(Self { key, event })
    }

    /// La notificación es de un solo disparo: hay que armarla de nuevo tras cada aviso.
    fn arm(&self) -> windows::core::Result<()> {
        // SAFETY: `key` y `event` son válidos mientras viva `self`.
        unsafe {
            RegNotifyChangeKeyValue(
                self.key,
                true,
                REG_NOTIFY_CHANGE_NAME | REG_NOTIFY_CHANGE_LAST_SET,
                Some(self.event),
                true,
            )
            .ok()
        }
    }
}

impl Drop for Watched {
    fn drop(&mut self) {
        // SAFETY: handles propios, abiertos en `open`.
        unsafe {
            let _ = RegCloseKey(self.key);
            let _ = CloseHandle(self.event);
        }
    }
}

fn run(hub: &ActivityHub) -> windows::core::Result<()> {
    let microphone = Watched::open("microphone")?;
    let camera = Watched::open("webcam")?;
    let mut last: Option<Privacy> = None;
    loop {
        // Armar *antes* de leer: un cambio entre la lectura y la espera no se pierde.
        microphone.arm()?;
        camera.arm()?;
        let privacy = Privacy {
            microphone: apps_in_use(microphone.key),
            camera: apps_in_use(camera.key),
        };
        if last.as_ref() != Some(&privacy) {
            hub.set_privacy(privacy.clone());
            last = Some(privacy);
        }
        // SAFETY: los eventos viven en `microphone` y `camera`.
        unsafe { WaitForMultipleObjects(&[microphone.event, camera.event], false, INFINITE) };
        thread::sleep(SETTLE);
    }
}

fn apps_in_use(capability: HKEY) -> Vec<String> {
    let mut apps = Vec::new();
    for name in subkeys(capability) {
        if name != NON_PACKAGED {
            if in_use(capability, &name) {
                apps.push(display_name(&name));
            }
            continue;
        }
        let mut desktop = HKEY::default();
        // SAFETY: out-param válido; se cierra abajo.
        let opened = unsafe {
            RegOpenKeyExW(
                capability,
                &HSTRING::from(NON_PACKAGED),
                None,
                KEY_READ,
                &mut desktop,
            )
        };
        if opened != ERROR_SUCCESS {
            continue;
        }
        for exe in subkeys(desktop) {
            if in_use(desktop, &exe) {
                apps.push(display_name(&exe));
            }
        }
        // SAFETY: clave abierta arriba.
        unsafe {
            let _ = RegCloseKey(desktop);
        }
    }
    apps.sort();
    apps.dedup();
    apps
}

fn subkeys(key: HKEY) -> Vec<String> {
    let mut names = Vec::new();
    let mut buffer = [0u16; 512];
    for index in 0.. {
        let mut len = buffer.len() as u32;
        // SAFETY: `buffer` y `len` describen un búfer válido de `len` caracteres.
        let result: WIN32_ERROR = unsafe {
            RegEnumKeyExW(
                key,
                index,
                Some(PWSTR(buffer.as_mut_ptr())),
                &mut len,
                None,
                None,
                None,
                None,
            )
        };
        if result == ERROR_NO_MORE_ITEMS {
            break;
        }
        if result == ERROR_SUCCESS {
            names.push(String::from_utf16_lossy(&buffer[..len as usize]));
        }
    }
    names
}

fn in_use(parent: HKEY, app: &str) -> bool {
    matches!(
        (read_qword(parent, app, "LastUsedTimeStart"), read_qword(parent, app, "LastUsedTimeStop")),
        (Some(start), Some(0)) if start != 0
    )
}

fn read_qword(parent: HKEY, subkey: &str, value: &str) -> Option<u64> {
    let mut data = 0u64;
    let mut size = size_of::<u64>() as u32;
    // SAFETY: `data` tiene exactamente `size` bytes y se pide un REG_QWORD.
    let result = unsafe {
        RegGetValueW(
            parent,
            &HSTRING::from(subkey),
            &HSTRING::from(value),
            RRF_RT_REG_QWORD,
            None,
            Some(&mut data as *mut u64 as *mut _),
            Some(&mut size),
        )
    };
    (result == ERROR_SUCCESS).then_some(data)
}

/// Nombre legible a partir de la subclave: `C:#Program Files#…#chrome.exe` (escritorio)
/// o `MSTeams_8wekyb3d8bbwe` (empaquetada).
fn display_name(raw: &str) -> String {
    const KNOWN: [(&str, &str); 6] = [
        ("msteams", "Teams"),
        ("ms-teams", "Teams"),
        ("windowscamera", "Cámara"),
        ("msedge", "Edge"),
        ("obs64", "OBS"),
        ("zoom", "Zoom"),
    ];
    let tail = raw.rsplit('#').next().unwrap_or(raw);
    let tail = tail
        .strip_suffix(".exe")
        .or_else(|| tail.strip_suffix(".EXE"))
        .unwrap_or(tail);
    // Paquetes: "Editor.App_hashdeeditor" → "App".
    let tail = tail.split('_').next().unwrap_or(tail);
    let tail = tail.rsplit('.').next().unwrap_or(tail);
    if let Some((_, name)) = KNOWN.iter().find(|(id, _)| tail.eq_ignore_ascii_case(id)) {
        return (*name).to_owned();
    }
    let mut chars = tail.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => raw.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::display_name;

    #[test]
    fn desktop_apps_use_their_executable() {
        assert_eq!(
            display_name("C:#Program Files#Google#Chrome#Application#chrome.exe"),
            "Chrome"
        );
        assert_eq!(
            display_name(r"C:#Program Files#obs-studio#bin#64bit#obs64.exe"),
            "OBS"
        );
        assert_eq!(
            display_name("C:#Users#me#AppData#Local#Discord#app#Discord.exe"),
            "Discord"
        );
    }

    #[test]
    fn packaged_apps_drop_publisher_and_hash() {
        assert_eq!(display_name("MSTeams_8wekyb3d8bbwe"), "Teams");
        assert_eq!(
            display_name("Microsoft.WindowsCamera_8wekyb3d8bbwe"),
            "Cámara"
        );
        assert_eq!(display_name("Contoso.Recorder_abc123"), "Recorder");
    }
}

//! Privacidad: qué apps están usando el micrófono o la cámara.
//!
//! - **Micrófono:** sesiones de audio *activas* en los dispositivos de captura (Core
//!   Audio). Cada sesión dice qué proceso la abrió.
//! - **Cámara:** `IMFSensorActivityMonitor` de Media Foundation, la API con la que Windows
//!   reporta qué proceso está transmitiendo desde un sensor de video.
//!
//! Antes se leía el registro `CapabilityAccessManager\ConsentStore` (`LastUsedTimeStop`
//! en 0 = en uso), pero en Windows 11 reciente ya no se actualiza: en la build 26300 las
//! entradas tenían semanas aunque la cámara y el micrófono estaban en uso. Ver ADR-0005.

use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Media::Audio::{
    AudioSessionStateActive, DEVICE_STATE_ACTIVE, IAudioSessionControl2, IAudioSessionManager2,
    IMMDeviceEnumerator, eCapture,
};
use windows::Win32::Media::MediaFoundation::{
    IMFSensorActivitiesReport, IMFSensorActivitiesReportCallback,
    IMFSensorActivitiesReportCallback_Impl, MF_API_VERSION, MF_SDK_VERSION,
    MFCreateSensorActivityMonitor, MFSTARTUP_LITE, MFStartup,
};
use windows::Win32::System::Com::{CLSCTX_ALL, COINIT_MULTITHREADED, CoInitializeEx};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::core::{Interface, PWSTR, Ref, implement};

use super::audio;
use crate::activity::{ActivityHub, Privacy};

/// Revisar las sesiones de captura es una consulta en memoria de Core Audio: cada
/// segundo no se nota y avisa casi al instante.
const POLL: Duration = Duration::from_secs(1);
/// Para procesos que no se dejan inspeccionar (elevados o del sistema): el indicador
/// debe aparecer aunque no sepamos el nombre.
const UNKNOWN_APP: &str = "Otra app";

type Pids = Arc<Mutex<Vec<u32>>>;

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

/// Recibe los reportes de actividad de cámara (en hilos de Media Foundation) y guarda
/// qué procesos están transmitiendo.
#[implement(IMFSensorActivitiesReportCallback)]
struct CameraListener {
    pids: Pids,
}

impl IMFSensorActivitiesReportCallback_Impl for CameraListener_Impl {
    fn OnActivitiesReport(
        &self,
        report: Ref<IMFSensorActivitiesReport>,
    ) -> windows::core::Result<()> {
        // Cada reporte es una foto completa de todos los sensores: reemplaza a la anterior.
        let pids = streaming_pids(report.ok()?)?;
        *self.pids.lock().unwrap_or_else(PoisonError::into_inner) = pids;
        Ok(())
    }
}

fn run(hub: &ActivityHub) -> windows::core::Result<()> {
    // SAFETY: inicializa COM y Media Foundation en este hilo antes de usar sus APIs.
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
        MFStartup((MF_SDK_VERSION << 16) | MF_API_VERSION, MFSTARTUP_LITE)?;
    }

    let camera_pids = Pids::default();
    let listener: IMFSensorActivitiesReportCallback = CameraListener {
        pids: camera_pids.clone(),
    }
    .into();
    // Si el monitor de cámara no está disponible, el micrófono sigue funcionando.
    // SAFETY: el callback vive en `listener` mientras viva el monitor (todo este hilo).
    let _camera_monitor = match unsafe { MFCreateSensorActivityMonitor(&listener) } {
        Ok(monitor) => unsafe { monitor.Start() }.map(|_| monitor).ok(),
        Err(err) => {
            eprintln!("monitor de cámara no disponible: {err}");
            None
        }
    };

    let enumerator = audio::device_enumerator()?;
    let own_pid = std::process::id();
    let mut last: Option<Privacy> = None;
    loop {
        let microphone = microphone_pids(&enumerator).unwrap_or_default();
        let camera = camera_pids
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        let privacy = Privacy {
            microphone: app_names(microphone, own_pid),
            camera: app_names(camera, own_pid),
        };
        if last.as_ref() != Some(&privacy) {
            hub.set_privacy(privacy.clone());
            last = Some(privacy);
        }
        thread::sleep(POLL);
    }
}

fn streaming_pids(report: &IMFSensorActivitiesReport) -> windows::core::Result<Vec<u32>> {
    let mut pids = Vec::new();
    // SAFETY: llamadas COM de solo lectura sobre el reporte que nos prestó el callback.
    unsafe {
        for sensor in 0..report.GetCount()? {
            let sensor = report.GetActivityReport(sensor)?;
            for process in 0..sensor.GetProcessCount()? {
                let activity = sensor.GetProcessActivity(process)?;
                if activity.GetStreamingState()?.as_bool() {
                    pids.push(activity.GetProcessId()?);
                }
            }
        }
    }
    Ok(pids)
}

/// Procesos con una sesión de audio activa en algún dispositivo de captura.
fn microphone_pids(enumerator: &IMMDeviceEnumerator) -> windows::core::Result<Vec<u32>> {
    let mut pids = Vec::new();
    // SAFETY: llamadas COM de solo lectura; cada interfaz se suelta al salir del scope.
    unsafe {
        let devices = enumerator.EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE)?;
        for index in 0..devices.GetCount()? {
            let manager: IAudioSessionManager2 = devices.Item(index)?.Activate(CLSCTX_ALL, None)?;
            let sessions = manager.GetSessionEnumerator()?;
            for session in 0..sessions.GetCount()? {
                let session = sessions.GetSession(session)?;
                if session.GetState()? != AudioSessionStateActive {
                    continue;
                }
                if let Ok(pid) = session.cast::<IAudioSessionControl2>()?.GetProcessId() {
                    pids.push(pid);
                }
            }
        }
    }
    Ok(pids)
}

/// Nombres únicos y ordenados. Fuera el PID 0 (sesiones del sistema) y la propia isla.
fn app_names(pids: Vec<u32>, own_pid: u32) -> Vec<String> {
    let mut names: Vec<String> = pids
        .into_iter()
        .filter(|&pid| pid != 0 && pid != own_pid)
        .map(|pid| process_name(pid).unwrap_or_else(|| UNKNOWN_APP.to_owned()))
        .collect();
    names.sort();
    names.dedup();
    names
}

fn process_name(pid: u32) -> Option<String> {
    // SAFETY: el handle se cierra antes de salir; el búfer y su tamaño van juntos.
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buffer = [0u16; 1024];
        let mut len = buffer.len() as u32;
        let queried = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(handle);
        queried.ok()?;
        Some(display_name(&String::from_utf16_lossy(
            &buffer[..len as usize],
        )))
    }
}

/// Nombre legible a partir de la ruta del ejecutable.
fn display_name(path: &str) -> String {
    const KNOWN: [(&str, &str); 6] = [
        ("msteams", "Teams"),
        ("ms-teams", "Teams"),
        ("windowscamera", "Cámara"),
        ("msedge", "Edge"),
        ("obs64", "OBS"),
        ("zoom", "Zoom"),
    ];
    let file = path.rsplit(['\\', '/']).next().unwrap_or(path);
    let stem = file
        .strip_suffix(".exe")
        .or_else(|| file.strip_suffix(".EXE"))
        .unwrap_or(file);
    if let Some((_, name)) = KNOWN.iter().find(|(id, _)| stem.eq_ignore_ascii_case(id)) {
        return (*name).to_owned();
    }
    let mut chars = stem.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => UNKNOWN_APP.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{UNKNOWN_APP, app_names, display_name};

    #[test]
    fn names_come_from_the_executable() {
        assert_eq!(
            display_name(r"C:\Program Files\Google\Chrome\Application\chrome.exe"),
            "Chrome"
        );
        assert_eq!(
            display_name(r"C:\Program Files\obs-studio\bin\64bit\obs64.exe"),
            "OBS"
        );
        assert_eq!(
            display_name(
                r"C:\Program Files\WindowsApps\Microsoft.WindowsCamera_2025.1.0.0_x64__8wekyb3d8bbwe\WindowsCamera.exe"
            ),
            "Cámara"
        );
        assert_eq!(display_name(r"C:\Apps\ms-teams.exe"), "Teams");
        assert_eq!(display_name(r"C:\Users\me\Discord\discord.exe"), "Discord");
        assert_eq!(display_name(""), UNKNOWN_APP);
    }

    #[test]
    fn own_process_and_system_sessions_are_ignored() {
        let own = std::process::id();
        assert!(app_names(vec![0, own], own).is_empty());
    }
}

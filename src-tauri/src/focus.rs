//! Llevar al frente la ventana de una app externa a partir de un PID de su árbol de
//! procesos. Sirve para que un clic en "Claude te necesita" te deje en la terminal
//! donde corre esa sesión.
//!
//! El hook de Claude Code manda el PID de su proceso padre, que es una consola sin
//! ventana. Subiendo por los padres (`claude.exe` → shell → terminal) se llega al primer
//! proceso que tiene una ventana principal: esa es la terminal.

use std::collections::HashMap;

use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GW_OWNER, GWL_EXSTYLE, GetWindow, GetWindowLongPtrW, GetWindowTextLengthW,
    GetWindowThreadProcessId, IsIconic, IsWindowVisible, SW_RESTORE, SetForegroundWindow,
    ShowWindow, WS_EX_TOOLWINDOW,
};
use windows::core::BOOL;

/// Suficiente para terminal → shell → claude → shell del hook → hook, con holgura.
const MAX_DEPTH: usize = 12;

/// Enfoca la ventana del primer ancestro de `pid` que tenga una. Devuelve si lo logró.
///
/// Windows solo deja que una app pase otra ventana al frente si recibió el último evento
/// de entrada; el clic del usuario en la isla cumple esa condición.
pub fn focus_process_window(pid: u32) -> bool {
    let parents = process_parents();
    let Some(window) = ancestors(pid, &parents).into_iter().find_map(main_window) else {
        return false;
    };
    // SAFETY: `window` es un HWND recién enumerado; si ya no existe, las llamadas fallan.
    unsafe {
        if IsIconic(window).as_bool() {
            let _ = ShowWindow(window, SW_RESTORE);
        }
        SetForegroundWindow(window).as_bool()
    }
}

/// La cadena `pid` → padre → abuelo… Con tope y sin repetir: Windows recicla PIDs, así
/// que un "padre" puede ser un proceso más nuevo que, a su vez, apunte de vuelta.
pub fn ancestors(pid: u32, parents: &HashMap<u32, u32>) -> Vec<u32> {
    let mut chain = Vec::new();
    let mut current = pid;
    while current != 0 && chain.len() < MAX_DEPTH && !chain.contains(&current) {
        chain.push(current);
        match parents.get(&current) {
            Some(&parent) => current = parent,
            None => break,
        }
    }
    chain
}

/// PID → PID del padre, de todos los procesos vivos.
fn process_parents() -> HashMap<u32, u32> {
    let mut parents = HashMap::new();
    // SAFETY: la foto se cierra al final; `entry` declara su tamaño como pide la API.
    unsafe {
        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return parents;
        };
        let mut entry = PROCESSENTRY32W {
            dwSize: size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut next = Process32FirstW(snapshot, &mut entry);
        while next.is_ok() {
            parents.insert(entry.th32ProcessID, entry.th32ParentProcessID);
            next = Process32NextW(snapshot, &mut entry);
        }
        let _ = CloseHandle(snapshot);
    }
    parents
}

/// La ventana principal de un proceso: visible, sin dueño, con título y que no sea una
/// ventana de herramienta (las que no salen en Alt+Tab).
fn main_window(pid: u32) -> Option<HWND> {
    struct Search {
        pid: u32,
        found: Option<HWND>,
    }

    unsafe extern "system" fn visit(window: HWND, data: LPARAM) -> BOOL {
        // SAFETY: `data` es el `&mut Search` de abajo, vivo durante toda la enumeración.
        let search = unsafe { &mut *(data.0 as *mut Search) };
        let mut owner_pid = 0u32;
        // SAFETY: consultas de solo lectura sobre un HWND que nos dio `EnumWindows`.
        let is_main = unsafe {
            GetWindowThreadProcessId(window, Some(&mut owner_pid));
            owner_pid == search.pid
                && IsWindowVisible(window).as_bool()
                && GetWindow(window, GW_OWNER).is_err()
                && GetWindowTextLengthW(window) > 0
                && GetWindowLongPtrW(window, GWL_EXSTYLE) & WS_EX_TOOLWINDOW.0 as isize == 0
        };
        if is_main {
            search.found = Some(window);
        }
        // Seguir enumerando solo mientras no se haya encontrado.
        (!is_main).into()
    }

    let mut search = Search { pid, found: None };
    // SAFETY: ver el comentario del callback. EnumWindows "falla" cuando el callback la
    // detiene; por eso se ignora su resultado y se mira `found`.
    let _ = unsafe { EnumWindows(Some(visit), LPARAM(&mut search as *mut Search as isize)) };
    search.found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walks_up_to_the_root() {
        let parents = HashMap::from([(40, 30), (30, 20), (20, 10)]);
        assert_eq!(ancestors(40, &parents), [40, 30, 20, 10]);
    }

    #[test]
    fn stops_on_cycles_and_pid_zero() {
        let cyclic = HashMap::from([(1, 2), (2, 1)]);
        assert_eq!(ancestors(1, &cyclic), [1, 2]);
        let to_idle = HashMap::from([(5, 0)]);
        assert_eq!(ancestors(5, &to_idle), [5]);
        assert!(ancestors(0, &HashMap::new()).is_empty());
    }

    #[test]
    fn depth_is_capped() {
        let parents: HashMap<u32, u32> = (1..100).map(|pid| (pid + 1, pid)).collect();
        assert_eq!(ancestors(100, &parents).len(), MAX_DEPTH);
    }

    /// Depende de la máquina: corre con `cargo test -- --ignored --nocapture` desde una
    /// terminal y debe encontrar la ventana de esa terminal.
    #[test]
    #[ignore]
    fn finds_the_terminal_of_this_process() {
        let parents = process_parents();
        let chain = ancestors(std::process::id(), &parents);
        let window = chain.iter().copied().find_map(main_window);
        println!("cadena de procesos: {chain:?}");
        println!("ventana encontrada: {window:?}");
        assert!(window.is_some());
    }
}

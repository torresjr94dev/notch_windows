//! Bloq Mayús: muchas laptops y teclados no tienen LED, así que la isla avisa al
//! activarlo o desactivarlo.
//!
//! Polling a 20 Hz a propósito: la alternativa por eventos es un hook global de teclado
//! (`WH_KEYBOARD_LL`), que ve *cada tecla* del sistema y se mete en el camino del input.
//! Para una sola tecla de estado, `GetKeyState` cada 50 ms es más barato y más seguro.

use std::thread;
use std::time::Duration;

use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CAPITAL};

use crate::activity::{Activity, ActivityData, ActivityHub};

const ACTIVITY_ID: &str = "system:caps-lock";
const TTL: Duration = Duration::from_millis(1500);
const POLL: Duration = Duration::from_millis(50);

pub fn spawn(hub: ActivityHub) -> std::io::Result<()> {
    thread::Builder::new()
        .name("keyboard-provider".into())
        .spawn(move || {
            // El estado inicial no se anuncia: solo los cambios.
            let mut previous = caps_lock_on();
            loop {
                thread::sleep(POLL);
                let current = caps_lock_on();
                if current != previous {
                    hub.upsert(Activity::transient(
                        ACTIVITY_ID,
                        ActivityData::CapsLock { on: current },
                        TTL,
                    ));
                    previous = current;
                }
            }
        })?;
    Ok(())
}

fn caps_lock_on() -> bool {
    // SAFETY: sin punteros. El bit bajo de `GetKeyState` es el estado de una tecla de
    // alternancia (encendida/apagada).
    unsafe { GetKeyState(i32::from(VK_CAPITAL.0)) & 1 != 0 }
}

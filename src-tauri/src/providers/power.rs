//! Batería: avisa al conectar o desconectar el cargador y cuando la batería cruza el
//! umbral de "baja". Sin batería (PC de escritorio) el provider termina de inmediato.

use std::thread;
use std::time::Duration;

use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

use crate::activity::{Activity, ActivityData, ActivityHub};

const ACTIVITY_ID: &str = "system:battery";
const TTL: Duration = Duration::from_millis(3500);
/// `GetSystemPowerStatus` es una lectura barata; consultarla cada segundo no se nota y
/// evita depender de una ventana para recibir `WM_POWERBROADCAST`.
const POLL: Duration = Duration::from_secs(1);
const LOW_PERCENT: u8 = 20;
/// `BatteryFlag`: el sistema no tiene batería.
const NO_SYSTEM_BATTERY: u8 = 128;
/// `BatteryLifePercent`: porcentaje desconocido.
const UNKNOWN_PERCENT: u8 = 255;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PowerState {
    percent: u8,
    charging: bool,
}

pub fn spawn(hub: ActivityHub) -> std::io::Result<()> {
    let Some(mut previous) = read() else {
        return Ok(());
    };
    thread::Builder::new()
        .name("power-provider".into())
        .spawn(move || {
            loop {
                thread::sleep(POLL);
                let Some(current) = read() else { continue };
                if let Some(data) = notice(previous, current) {
                    hub.upsert(Activity::transient(ACTIVITY_ID, data, TTL));
                }
                previous = current;
            }
        })?;
    Ok(())
}

fn read() -> Option<PowerState> {
    let mut status = SYSTEM_POWER_STATUS::default();
    // SAFETY: `status` es un out-param válido.
    unsafe { GetSystemPowerStatus(&mut status) }.ok()?;
    if status.BatteryFlag & NO_SYSTEM_BATTERY != 0 || status.BatteryLifePercent == UNKNOWN_PERCENT {
        return None;
    }
    Some(PowerState {
        percent: status.BatteryLifePercent.min(100),
        charging: status.ACLineStatus == 1,
    })
}

/// Qué aviso mostrar para un cambio de estado, si alguno.
fn notice(previous: PowerState, current: PowerState) -> Option<ActivityData> {
    let low = !current.charging && current.percent <= LOW_PERCENT;
    let crossed_low = !current.charging && previous.percent > LOW_PERCENT && low;
    (current.charging != previous.charging || crossed_low).then_some(ActivityData::Battery {
        percent: current.percent,
        charging: current.charging,
        low,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(percent: u8, charging: bool) -> PowerState {
        PowerState { percent, charging }
    }

    fn battery(percent: u8, charging: bool, low: bool) -> Option<ActivityData> {
        Some(ActivityData::Battery {
            percent,
            charging,
            low,
        })
    }

    #[test]
    fn plugging_and_unplugging_the_charger_notifies() {
        assert_eq!(
            notice(state(76, false), state(76, true)),
            battery(76, true, false)
        );
        assert_eq!(
            notice(state(76, true), state(76, false)),
            battery(76, false, false)
        );
    }

    #[test]
    fn crossing_the_low_threshold_notifies_once() {
        assert_eq!(
            notice(state(21, false), state(20, false)),
            battery(20, false, true)
        );
        assert_eq!(notice(state(20, false), state(19, false)), None);
    }

    #[test]
    fn unplugging_while_low_is_flagged_low() {
        assert_eq!(
            notice(state(15, true), state(15, false)),
            battery(15, false, true)
        );
    }

    #[test]
    fn steady_state_is_silent() {
        assert_eq!(notice(state(50, false), state(49, false)), None);
        assert_eq!(notice(state(50, true), state(51, true)), None);
    }
}

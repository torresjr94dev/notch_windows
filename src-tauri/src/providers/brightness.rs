//! Brillo: el evento WMI `WmiMonitorBrightnessEvent`, que Windows dispara cuando cambia
//! el brillo del panel integrado (laptops). Los monitores externos no lo reportan, así
//! que en una PC de escritorio este provider simplemente nunca publica nada.

use std::thread;
use std::time::Duration;

use serde::Deserialize;
use wmi::WMIConnection;

use crate::activity::{Activity, ActivityData, ActivityHub};

const ACTIVITY_ID: &str = "system:brightness";
const TTL: Duration = Duration::from_millis(1800);

#[derive(Deserialize)]
#[serde(rename = "WmiMonitorBrightnessEvent", rename_all = "PascalCase")]
struct BrightnessEvent {
    /// Porcentaje de 0 a 100.
    brightness: u8,
}

pub fn spawn(hub: ActivityHub) -> std::io::Result<()> {
    thread::Builder::new()
        .name("brightness-provider".into())
        .spawn(move || {
            if let Err(err) = run(&hub) {
                // Sin panel integrado (o sin el proveedor WMI) no hay nada que vigilar.
                eprintln!("brillo no disponible: {err}");
            }
        })?;
    Ok(())
}

fn run(hub: &ActivityHub) -> wmi::WMIResult<()> {
    let connection = WMIConnection::with_namespace_path("root\\WMI")?;
    // El iterador bloquea este hilo hasta que llega cada evento: cero polling.
    for event in connection.notification::<BrightnessEvent>()? {
        let event = event?;
        hub.upsert(Activity::transient(
            ACTIVITY_ID,
            ActivityData::Brightness {
                level: f32::from(event.brightness.min(100)) / 100.0,
            },
            TTL,
        ));
    }
    Ok(())
}

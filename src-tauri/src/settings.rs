//! Preferencias del usuario, persistidas como JSON en el directorio de configuración de
//! la app (`%APPDATA%\dev.notchwindows.island\settings.json`).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};

/// En qué pantalla(s) vive la isla.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", content = "id", rename_all = "camelCase")]
pub enum DisplayChoice {
    /// El monitor principal de Windows, aunque cambie.
    #[default]
    Primary,
    /// Una isla por monitor conectado.
    All,
    /// Un monitor concreto, por su id estable (ver `island::displays`). Si se
    /// desconecta, la isla se va al principal hasta que regrese.
    Monitor(String),
}

/// `#[serde(default)]` hace que un archivo de una versión anterior (sin los campos
/// nuevos) se lea sin error, con los valores por defecto para lo que falte.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub display: DisplayChoice,
    /// Barras de audio en Now Playing. Implica capturar el audio del sistema mientras
    /// suena música, así que el usuario puede apagarlo.
    pub visualizer: bool,
    /// Puerto de la API local (`127.0.0.1`). Configurable por si otro programa lo ocupa.
    pub api_port: u16,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            display: DisplayChoice::Primary,
            visualizer: true,
            api_port: crate::api::DEFAULT_PORT,
        }
    }
}

pub struct SettingsStore {
    path: PathBuf,
    current: Mutex<Settings>,
}

impl SettingsStore {
    /// Lee las preferencias. Un archivo faltante o corrupto no debe impedir que la app
    /// arranque: se usan los valores por defecto y se sobrescribe al siguiente cambio.
    pub fn load(path: PathBuf) -> Self {
        let current = fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        Self {
            path,
            current: Mutex::new(current),
        }
    }

    pub fn get(&self) -> Settings {
        self.lock().clone()
    }

    /// Aplica el cambio y lo guarda en disco.
    pub fn update(&self, change: impl FnOnce(&mut Settings)) {
        let snapshot = {
            let mut current = self.lock();
            change(&mut current);
            current.clone()
        };
        if let Err(err) = write_atomically(&self.path, &snapshot) {
            eprintln!("no se pudieron guardar las preferencias: {err}");
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Settings> {
        self.current.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Escribe a un temporal y luego renombra: si la app muere a medio guardar, el archivo
/// anterior queda intacto en vez de truncado.
fn write_atomically(path: &Path, settings: &Settings) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(settings)?)?;
    fs::rename(tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_choice_round_trips_as_tagged_json() {
        let cases = [
            (DisplayChoice::Primary, r#"{"mode":"primary"}"#),
            (DisplayChoice::All, r#"{"mode":"all"}"#),
            (
                DisplayChoice::Monitor("DEL40F7#1".into()),
                r#"{"mode":"monitor","id":"DEL40F7#1"}"#,
            ),
        ];
        for (choice, json) in cases {
            assert_eq!(serde_json::to_string(&choice).unwrap(), json);
            assert_eq!(serde_json::from_str::<DisplayChoice>(json).unwrap(), choice);
        }
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let settings: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn corrupt_file_loads_defaults_and_update_persists() {
        let dir = std::env::temp_dir().join(format!("notch-settings-{}", std::process::id()));
        let path = dir.join("settings.json");
        fs::create_dir_all(&dir).unwrap();
        fs::write(&path, "{ esto no es json").unwrap();

        let store = SettingsStore::load(path.clone());
        assert_eq!(store.get(), Settings::default());

        store.update(|s| s.display = DisplayChoice::All);
        let reloaded = SettingsStore::load(path);
        assert_eq!(reloaded.get().display, DisplayChoice::All);

        fs::remove_dir_all(dir).unwrap();
    }
}

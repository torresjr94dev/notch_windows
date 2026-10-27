//! Monitores conectados y las reglas (puras, con tests) para decidir dónde va una isla.

use super::geometry::MonitorArea;
use crate::settings::DisplayChoice;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Display {
    /// Id estable entre reinicios: la ruta de dispositivo del monitor, que incluye modelo
    /// y puerto. `\\.\DISPLAYn` no sirve porque Windows renumera al reconectar.
    pub id: String,
    /// Modelo según el EDID del monitor ("DELL U3419W"). Puede venir vacío.
    pub name: String,
    /// Panel integrado de una laptop.
    pub internal: bool,
    /// `HMONITOR`. Solo es válido mientras no cambie la topología de pantallas.
    pub handle: isize,
    pub area: MonitorArea,
    pub primary: bool,
}

/// Monitores donde debe haber isla según la preferencia del usuario.
pub fn targets<'a>(choice: &DisplayChoice, displays: &'a [Display]) -> Vec<&'a Display> {
    let primary = || displays.iter().find(|d| d.primary).or(displays.first());
    match choice {
        DisplayChoice::All => displays.iter().collect(),
        DisplayChoice::Primary => primary().into_iter().collect(),
        // Si el monitor elegido no está conectado, la isla se va al principal; la
        // preferencia no se toca, así que regresa sola cuando se reconecte.
        DisplayChoice::Monitor(id) => displays
            .iter()
            .find(|d| &d.id == id)
            .or_else(primary)
            .into_iter()
            .collect(),
    }
}

/// Etiquetas para el menú, en el mismo orden: "DELL U3419W · 3440×1440". Sin nombre
/// EDID se usa "Pantalla integrada" o "Monitor N"; los modelos repetidos se numeran.
pub fn labels(displays: &[Display]) -> Vec<String> {
    let names: Vec<String> = displays
        .iter()
        .enumerate()
        .map(|(i, d)| match (d.name.is_empty(), d.internal) {
            (false, _) => d.name.clone(),
            (true, true) => "Pantalla integrada".into(),
            (true, false) => format!("Monitor {}", i + 1),
        })
        .collect();

    names
        .iter()
        .zip(displays)
        .enumerate()
        .map(|(i, (name, d))| {
            let name = if names.iter().filter(|n| *n == name).count() > 1 {
                let nth = names[..=i].iter().filter(|n| *n == name).count();
                format!("{name} ({nth})")
            } else {
                name.clone()
            };
            format!("{name} · {}×{}", d.area.bounds.width, d.area.bounds.height)
        })
        .collect()
}

/// Qué hacer con las islas existentes para cubrir los monitores destino.
#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    /// Mover la isla `index` a otro monitor. Reusar la ventana evita recargar el
    /// WebView y el parpadeo, p. ej. cuando cambia cuál es el monitor principal.
    Reassign {
        index: usize,
        display: String,
    },
    Remove {
        index: usize,
    },
    Create {
        display: String,
    },
}

pub fn plan(existing: &[&str], targets: &[&str]) -> Vec<Action> {
    let mut orphans = existing
        .iter()
        .enumerate()
        .filter(|(_, id)| !targets.contains(id))
        .map(|(index, _)| index);
    let mut actions = Vec::new();
    for target in targets.iter().filter(|id| !existing.contains(id)) {
        actions.push(match orphans.next() {
            Some(index) => Action::Reassign {
                index,
                display: (*target).to_owned(),
            },
            None => Action::Create {
                display: (*target).to_owned(),
            },
        });
    }
    actions.extend(orphans.map(|index| Action::Remove { index }));
    actions
}

/// Dónde hay algo en pantalla completa.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fullscreen {
    None,
    /// En el monitor con este `HMONITOR`.
    Monitor(isize),
    /// Modo presentación de Windows: aplica a todas las pantallas.
    Everywhere,
}

impl Fullscreen {
    pub fn covers(self, monitor: isize) -> bool {
        match self {
            Fullscreen::None => false,
            Fullscreen::Monitor(handle) => handle == monitor,
            Fullscreen::Everywhere => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::island::geometry::Rect;

    fn display(id: &str, name: &str, x: i32, primary: bool) -> Display {
        let bounds = Rect {
            x,
            y: 0,
            width: 1920,
            height: 1080,
        };
        Display {
            id: id.into(),
            name: name.into(),
            internal: false,
            handle: x as isize,
            area: MonitorArea {
                bounds,
                work: bounds,
            },
            primary,
        }
    }

    fn ids<'a>(displays: &[&'a Display]) -> Vec<&'a str> {
        displays.iter().map(|d| d.id.as_str()).collect()
    }

    #[test]
    fn targets_follow_the_choice() {
        let all = [display("a", "", 0, false), display("b", "", 1920, true)];
        assert_eq!(ids(&targets(&DisplayChoice::Primary, &all)), ["b"]);
        assert_eq!(ids(&targets(&DisplayChoice::All, &all)), ["a", "b"]);
        assert_eq!(
            ids(&targets(&DisplayChoice::Monitor("a".into()), &all)),
            ["a"]
        );
    }

    #[test]
    fn missing_monitor_falls_back_to_primary() {
        let all = [display("a", "", 0, false), display("b", "", 1920, true)];
        assert_eq!(
            ids(&targets(&DisplayChoice::Monitor("gone".into()), &all)),
            ["b"]
        );
    }

    #[test]
    fn no_displays_means_no_targets() {
        assert!(targets(&DisplayChoice::Primary, &[]).is_empty());
    }

    #[test]
    fn labels_name_and_number_monitors() {
        let mut laptop = display("c", "", 3840, false);
        laptop.internal = true;
        let all = [
            display("a", "DELL U2720Q", 0, true),
            display("b", "DELL U2720Q", 1920, false),
            laptop,
            display("d", "", 5760, false),
        ];
        assert_eq!(
            labels(&all),
            [
                "DELL U2720Q (1) · 1920×1080",
                "DELL U2720Q (2) · 1920×1080",
                "Pantalla integrada · 1920×1080",
                "Monitor 4 · 1920×1080",
            ]
        );
    }

    #[test]
    fn plan_reuses_orphan_windows_before_creating() {
        assert_eq!(
            plan(&["a"], &["b"]),
            [Action::Reassign {
                index: 0,
                display: "b".into()
            }]
        );
        assert_eq!(
            plan(&["a"], &["a", "b"]),
            [Action::Create {
                display: "b".into()
            }]
        );
        assert_eq!(plan(&["a", "b"], &["b"]), [Action::Remove { index: 0 }]);
        assert!(plan(&["a", "b"], &["a", "b"]).is_empty());
    }

    #[test]
    fn fullscreen_covers_only_its_monitor() {
        assert!(!Fullscreen::None.covers(1));
        assert!(Fullscreen::Monitor(1).covers(1));
        assert!(!Fullscreen::Monitor(1).covers(2));
        assert!(Fullscreen::Everywhere.covers(2));
    }
}

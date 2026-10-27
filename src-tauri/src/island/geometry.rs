//! Geometría pura (sin Win32) para poder probarla con tests unitarios.
//! Todas las medidas están en píxeles físicos.

use serde::Deserialize;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    /// ¿El punto cae dentro del rectángulo agrandado `margin` píxeles por lado?
    ///
    /// El margen da histéresis: entrar exige tocar la pastilla, pero salir exige
    /// alejarse un poco. Sin eso, un cursor quieto justo en el borde haría parpadear
    /// la isla entre expandida y compacta.
    pub fn contains(&self, px: i32, py: i32, margin: i32) -> bool {
        if self.width <= 0 || self.height <= 0 {
            return false;
        }
        px >= self.x - margin
            && px < self.x + self.width + margin
            && py >= self.y - margin
            && py < self.y + self.height + margin
    }
}

/// Área de un monitor: `bounds` es la pantalla completa y `work` lo que deja libre
/// la barra de tareas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonitorArea {
    pub bounds: Rect,
    pub work: Rect,
}

/// Esquina superior izquierda donde va la ventana de la isla.
///
/// Se centra contra la pantalla completa (no contra el área de trabajo) para que una
/// barra de tareas vertical no la descentre, pero se pega al tope del área de trabajo
/// para no quedar encima de una barra de tareas superior.
pub fn island_origin(area: MonitorArea, window_width: i32) -> (i32, i32) {
    (
        area.bounds.x + (area.bounds.width - window_width) / 2,
        area.work.y,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL_HD: Rect = Rect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };

    const PILL: Rect = Rect {
        x: 100,
        y: 0,
        width: 200,
        height: 50,
    };

    #[test]
    fn contains_is_half_open() {
        assert!(PILL.contains(100, 0, 0));
        assert!(PILL.contains(299, 49, 0));
        assert!(!PILL.contains(300, 10, 0));
        assert!(!PILL.contains(150, 50, 0));
    }

    #[test]
    fn margin_extends_every_side() {
        assert!(!PILL.contains(95, 10, 0));
        assert!(PILL.contains(95, 10, 8));
        assert!(PILL.contains(307, 57, 8));
        assert!(!PILL.contains(308, 10, 8));
    }

    #[test]
    fn empty_rect_contains_nothing() {
        assert!(!Rect::default().contains(0, 0, 8));
    }

    #[test]
    fn centers_on_primary_monitor_with_bottom_taskbar() {
        let area = MonitorArea {
            bounds: FULL_HD,
            work: Rect {
                height: 1032,
                ..FULL_HD
            },
        };
        assert_eq!(island_origin(area, 840), (540, 0));
    }

    #[test]
    fn stays_below_a_top_taskbar() {
        let area = MonitorArea {
            bounds: FULL_HD,
            work: Rect {
                y: 48,
                height: 1032,
                ..FULL_HD
            },
        };
        assert_eq!(island_origin(area, 840), (540, 48));
    }

    #[test]
    fn ignores_a_vertical_taskbar_when_centering() {
        let area = MonitorArea {
            bounds: FULL_HD,
            work: Rect {
                x: 60,
                width: 1860,
                ..FULL_HD
            },
        };
        assert_eq!(island_origin(area, 840), (540, 0));
    }

    #[test]
    fn handles_monitors_with_negative_origin() {
        let area = MonitorArea {
            bounds: Rect {
                x: -2560,
                y: -200,
                width: 2560,
                height: 1440,
            },
            work: Rect {
                x: -2560,
                y: -200,
                width: 2560,
                height: 1392,
            },
        };
        assert_eq!(island_origin(area, 1120), (-1840, -200));
    }
}

//! El tablero de actividades: lógica pura de arbitraje, sin hilos ni Tauri, para
//! poder probarla con tiempos controlados.

use std::mem::discriminant;
use std::time::Instant;

use serde::Serialize;

use super::{Activity, Privacy, Update};

/// Lo que dibujan las islas.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IslandView {
    /// Dueña de la pastilla.
    pub primary: Option<Activity>,
    /// Va en la burbuja del split. Nunca es transitoria ni del mismo tipo que la primaria.
    pub secondary: Option<Activity>,
    pub privacy: Privacy,
}

struct Entry {
    activity: Activity,
    /// Para desempatar prioridades iguales: gana la más reciente.
    updated_at: Instant,
    expires_at: Option<Instant>,
}

#[derive(Default)]
pub struct Board {
    entries: Vec<Entry>,
    privacy: Privacy,
}

impl Board {
    pub fn apply(&mut self, update: Update, now: Instant) {
        match update {
            Update::Privacy(privacy) => self.privacy = privacy,
            Update::Remove(id) => self.entries.retain(|e| e.activity.id != id),
            Update::Upsert(activity) => {
                let existing = self
                    .entries
                    .iter_mut()
                    .find(|e| e.activity.id == activity.id);
                match existing {
                    // Republicar lo mismo (los providers reintentan seguido) no debe
                    // reiniciar su TTL: una canción en pausa caduca aunque Windows siga
                    // avisando que está en pausa.
                    Some(entry) if entry.activity == activity => {}
                    Some(entry) => {
                        entry.expires_at = activity.ttl.map(|ttl| now + ttl);
                        entry.activity = activity;
                        entry.updated_at = now;
                    }
                    None => self.entries.push(Entry {
                        expires_at: activity.ttl.map(|ttl| now + ttl),
                        activity,
                        updated_at: now,
                    }),
                }
            }
        }
    }

    pub fn expire(&mut self, now: Instant) {
        self.entries
            .retain(|e| e.expires_at.is_none_or(|expires| expires > now));
    }

    pub fn next_expiry(&self) -> Option<Instant> {
        self.entries.iter().filter_map(|e| e.expires_at).min()
    }

    pub fn view(&self) -> IslandView {
        let mut ranked: Vec<&Entry> = self.entries.iter().collect();
        ranked.sort_by(|a, b| {
            b.activity
                .priority
                .cmp(&a.activity.priority)
                .then(b.updated_at.cmp(&a.updated_at))
        });

        let primary = ranked.first().map(|e| e.activity.clone());
        let secondary = ranked
            .iter()
            .skip(1)
            .map(|e| &e.activity)
            .find(|a| {
                !a.transient
                    && primary
                        .as_ref()
                        .is_none_or(|p| discriminant(&p.data) != discriminant(&a.data))
            })
            .cloned();
        IslandView {
            primary,
            secondary,
            privacy: self.privacy.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::activity::{ActivityData, NowPlaying};

    fn media(id: &str, priority: u8, title: &str) -> Activity {
        Activity {
            id: id.into(),
            priority,
            transient: false,
            data: ActivityData::NowPlaying(NowPlaying {
                title: title.into(),
                artist: String::new(),
                album: String::new(),
                app: String::new(),
                artwork_id: None,
                playing: true,
                position_ms: 0,
                duration_ms: 0,
                sampled_at_ms: 0,
            }),
            ttl: None,
        }
    }

    fn volume(level: f32, ttl_ms: u64) -> Activity {
        Activity::transient(
            "volume",
            ActivityData::Volume {
                level,
                muted: false,
            },
            Duration::from_millis(ttl_ms),
        )
    }

    fn ids(view: &IslandView) -> (Option<&str>, Option<&str>) {
        (
            view.primary.as_ref().map(|a| a.id.as_str()),
            view.secondary.as_ref().map(|a| a.id.as_str()),
        )
    }

    #[test]
    fn empty_board_has_empty_view() {
        assert_eq!(Board::default().view(), IslandView::default());
    }

    #[test]
    fn highest_priority_wins_and_newest_breaks_ties() {
        let t0 = Instant::now();
        let mut board = Board::default();
        board.apply(Update::Upsert(media("a", 50, "a")), t0);
        board.apply(
            Update::Upsert(media("b", 50, "b")),
            t0 + Duration::from_secs(1),
        );
        assert_eq!(ids(&board.view()).0, Some("b"));

        board.apply(
            Update::Upsert(media("c", 90, "c")),
            t0 + Duration::from_secs(2),
        );
        assert_eq!(ids(&board.view()).0, Some("c"));
    }

    #[test]
    fn transient_takes_the_pill_and_pushes_music_to_the_bubble() {
        let t0 = Instant::now();
        let mut board = Board::default();
        board.apply(Update::Upsert(media("music", 50, "song")), t0);
        board.apply(Update::Upsert(volume(0.5, 2000)), t0);
        assert_eq!(ids(&board.view()), (Some("volume"), Some("music")));
    }

    #[test]
    fn secondary_skips_transients_and_same_kind() {
        let t0 = Instant::now();
        let mut board = Board::default();
        board.apply(Update::Upsert(media("music", 50, "song")), t0);
        board.apply(Update::Upsert(media("other", 40, "x")), t0);
        assert_eq!(ids(&board.view()), (Some("music"), None));

        board.apply(Update::Upsert(media("music", 5, "song")), t0);
        board.apply(Update::Upsert(volume(0.5, 2000)), t0);
        // Primaria: el volumen. La burbuja no puede ser otro transitorio y se queda
        // con la mejor de las persistentes.
        assert_eq!(ids(&board.view()), (Some("volume"), Some("other")));
    }

    #[test]
    fn privacy_rides_along_with_any_activity() {
        let t0 = Instant::now();
        let mut board = Board::default();
        let privacy = Privacy {
            microphone: vec!["Teams".into()],
            camera: vec![],
        };
        board.apply(Update::Privacy(privacy.clone()), t0);
        board.apply(Update::Upsert(media("music", 50, "song")), t0);
        assert_eq!(board.view().privacy, privacy);
    }

    #[test]
    fn transients_expire() {
        let t0 = Instant::now();
        let mut board = Board::default();
        board.apply(Update::Upsert(media("music", 50, "song")), t0);
        board.apply(Update::Upsert(volume(0.5, 2000)), t0);
        assert_eq!(board.next_expiry(), Some(t0 + Duration::from_secs(2)));

        board.expire(t0 + Duration::from_millis(1999));
        assert_eq!(ids(&board.view()).0, Some("volume"));
        board.expire(t0 + Duration::from_secs(2));
        assert_eq!(ids(&board.view()).0, Some("music"));
        assert_eq!(board.next_expiry(), None);
    }

    #[test]
    fn identical_republish_keeps_ttl_but_changes_restart_it() {
        let t0 = Instant::now();
        let mut board = Board::default();
        board.apply(Update::Upsert(volume(0.5, 2000)), t0);
        board.apply(
            Update::Upsert(volume(0.5, 2000)),
            t0 + Duration::from_secs(1),
        );
        assert_eq!(board.next_expiry(), Some(t0 + Duration::from_secs(2)));

        board.apply(
            Update::Upsert(volume(0.6, 2000)),
            t0 + Duration::from_secs(1),
        );
        assert_eq!(board.next_expiry(), Some(t0 + Duration::from_secs(3)));
    }

    #[test]
    fn remove_drops_the_activity() {
        let t0 = Instant::now();
        let mut board = Board::default();
        board.apply(Update::Upsert(media("music", 50, "song")), t0);
        board.apply(Update::Remove("music".into()), t0);
        assert_eq!(board.view(), IslandView::default());
    }
}

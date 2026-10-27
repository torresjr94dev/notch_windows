//! Rutas, autenticación y validación de la API: lógica pura, probada sin sockets.

use std::time::Duration;

use serde::Deserialize;

use crate::activity::{Activity, ActivityData, LiveActivity, LiveIcon, LiveState, priority};

pub const MAX_BODY: usize = 16 * 1024;
const MAX_ID: usize = 64;
const MAX_TITLE: usize = 80;
const MAX_SUBTITLE: usize = 120;
const MAX_TTL: Duration = Duration::from_secs(24 * 60 * 60);
/// Cuánto se queda un "terminó" o "falló" si quien lo publica no dice otra cosa.
const DONE_TTL: Duration = Duration::from_secs(6);
/// Prefijo de las actividades externas: no pueden pisar las del sistema ("media"…).
pub const ID_PREFIX: &str = "live:";

#[derive(Debug, PartialEq, Eq)]
pub enum Route {
    Health,
    Put(String),
    Delete(String),
}

#[derive(Debug, PartialEq, Eq)]
pub enum ApiError {
    BadRequest(String),
    Unauthorized,
    Forbidden,
    NotFound,
    MethodNotAllowed,
    PayloadTooLarge,
}

impl ApiError {
    pub fn status(&self) -> u16 {
        match self {
            Self::BadRequest(_) => 400,
            Self::Unauthorized => 401,
            Self::Forbidden => 403,
            Self::NotFound => 404,
            Self::MethodNotAllowed => 405,
            Self::PayloadTooLarge => 413,
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::BadRequest(reason) => reason.clone(),
            Self::Unauthorized => {
                "falta el header `Authorization: Bearer <token>` o es inválido".into()
            }
            Self::Forbidden => "la API no acepta peticiones de navegadores".into(),
            Self::NotFound => "ruta desconocida".into(),
            Self::MethodNotAllowed => "método no permitido en esta ruta".into(),
            Self::PayloadTooLarge => format!("el cuerpo excede {MAX_BODY} bytes"),
        }
    }
}

pub fn route(method: &str, url: &str) -> Result<Route, ApiError> {
    let path = url.split('?').next().unwrap_or(url);
    let Some(rest) = path.strip_prefix("/v1/") else {
        return Err(ApiError::NotFound);
    };
    if rest == "health" {
        return match method {
            "GET" => Ok(Route::Health),
            _ => Err(ApiError::MethodNotAllowed),
        };
    }
    let Some(id) = rest.strip_prefix("activities/") else {
        return Err(ApiError::NotFound);
    };
    validate_id(id)?;
    match method {
        "PUT" => Ok(Route::Put(id.to_owned())),
        "DELETE" => Ok(Route::Delete(id.to_owned())),
        _ => Err(ApiError::MethodNotAllowed),
    }
}

fn validate_id(id: &str) -> Result<(), ApiError> {
    let valid = !id.is_empty()
        && id.len() <= MAX_ID
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':'));
    if valid {
        Ok(())
    } else {
        Err(ApiError::BadRequest(format!(
            "el id debe tener de 1 a {MAX_ID} caracteres: letras, números, '-', '_', '.' o ':'"
        )))
    }
}

/// `true` si el header trae exactamente `Bearer <token>`.
pub fn authorized(header: Option<&str>, token: &str) -> bool {
    header
        .and_then(|value| value.strip_prefix("Bearer "))
        .is_some_and(|given| constant_time_eq(given.as_bytes(), token.as_bytes()))
}

/// Comparación sin salida temprana: el tiempo de respuesta no revela cuántos
/// caracteres del token acertó un atacante.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct LiveRequest {
    title: String,
    subtitle: Option<String>,
    #[serde(default = "running")]
    state: LiveState,
    progress: Option<f32>,
    #[serde(default)]
    icon: LiveIcon,
    ttl_seconds: Option<u64>,
    /// PID cuya ventana (o la de su primer ancestro con ventana) se enfoca al hacer clic.
    focus_pid: Option<u32>,
}

fn running() -> LiveState {
    LiveState::Running
}

/// Convierte el cuerpo JSON de un `PUT` en la actividad que se publica.
pub fn activity(id: &str, body: &[u8]) -> Result<Activity, ApiError> {
    let request: LiveRequest =
        serde_json::from_slice(body).map_err(|err| ApiError::BadRequest(err.to_string()))?;

    let title = request.title.trim();
    if title.is_empty() || title.chars().count() > MAX_TITLE {
        return Err(ApiError::BadRequest(format!(
            "`title` debe tener de 1 a {MAX_TITLE} caracteres"
        )));
    }
    let subtitle = request
        .subtitle
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty());
    if subtitle
        .as_ref()
        .is_some_and(|s| s.chars().count() > MAX_SUBTITLE)
    {
        return Err(ApiError::BadRequest(format!(
            "`subtitle` admite hasta {MAX_SUBTITLE} caracteres"
        )));
    }
    if request.progress.is_some_and(|p| !p.is_finite()) {
        return Err(ApiError::BadRequest("`progress` debe ser un número".into()));
    }
    if request.focus_pid == Some(0) {
        return Err(ApiError::BadRequest("`focusPid` no puede ser 0".into()));
    }
    let ttl = match request.ttl_seconds {
        Some(seconds) => Some(Duration::from_secs(seconds).min(MAX_TTL)),
        None if matches!(request.state, LiveState::Success | LiveState::Error) => Some(DONE_TTL),
        None => None,
    };

    Ok(Activity {
        id: format!("{ID_PREFIX}{id}"),
        priority: match request.state {
            LiveState::Attention => priority::ATTENTION,
            _ => priority::LIVE,
        },
        transient: false,
        data: ActivityData::Live(LiveActivity {
            title: title.to_owned(),
            subtitle,
            state: request.state,
            progress: request.progress.map(|p| p.clamp(0.0, 1.0)),
            icon: request.icon,
            focus_pid: request.focus_pid,
            focusable: request.focus_pid.is_some(),
        }),
        ttl,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes() {
        assert_eq!(route("GET", "/v1/health"), Ok(Route::Health));
        assert_eq!(
            route("PUT", "/v1/activities/claude-1a2b"),
            Ok(Route::Put("claude-1a2b".into()))
        );
        assert_eq!(
            route("DELETE", "/v1/activities/ci:main?x=1"),
            Ok(Route::Delete("ci:main".into()))
        );
        assert_eq!(
            route("POST", "/v1/activities/a"),
            Err(ApiError::MethodNotAllowed)
        );
        assert_eq!(route("PUT", "/v1/health"), Err(ApiError::MethodNotAllowed));
        assert_eq!(route("GET", "/"), Err(ApiError::NotFound));
        assert_eq!(route("GET", "/v1/other"), Err(ApiError::NotFound));
    }

    #[test]
    fn ids_are_validated() {
        assert!(matches!(
            route("PUT", "/v1/activities/"),
            Err(ApiError::BadRequest(_))
        ));
        assert!(matches!(
            route("PUT", "/v1/activities/../../etc"),
            Err(ApiError::BadRequest(_))
        ));
        assert!(matches!(
            route("PUT", &format!("/v1/activities/{}", "a".repeat(65))),
            Err(ApiError::BadRequest(_))
        ));
    }

    #[test]
    fn auth_requires_exact_bearer_token() {
        assert!(authorized(Some("Bearer abc123"), "abc123"));
        assert!(!authorized(Some("Bearer abc124"), "abc123"));
        assert!(!authorized(Some("Bearer abc12"), "abc123"));
        assert!(!authorized(Some("abc123"), "abc123"));
        assert!(!authorized(None, "abc123"));
    }

    #[test]
    fn minimal_body_is_a_running_activity() {
        let activity = activity("build", br#"{"title":" Build "}"#).unwrap();
        assert_eq!(activity.id, "live:build");
        assert_eq!(activity.priority, priority::LIVE);
        assert_eq!(activity.ttl, None);
        let ActivityData::Live(live) = activity.data else {
            panic!("debe ser Live")
        };
        assert_eq!(live.title, "Build");
        assert_eq!(live.state, LiveState::Running);
        assert_eq!(live.icon, LiveIcon::Generic);
    }

    #[test]
    fn attention_outranks_music_and_done_states_expire() {
        let attention = activity("c", br#"{"title":"t","state":"attention"}"#).unwrap();
        assert_eq!(attention.priority, priority::ATTENTION);
        let done = activity("c", br#"{"title":"t","state":"success"}"#).unwrap();
        assert_eq!(done.ttl, Some(DONE_TTL));
        let custom = activity("c", br#"{"title":"t","state":"error","ttlSeconds":30}"#).unwrap();
        assert_eq!(custom.ttl, Some(Duration::from_secs(30)));
    }

    #[test]
    fn focus_pid_makes_the_activity_focusable_without_exposing_the_pid() {
        let published = activity("c", br#"{"title":"t","focusPid":4242}"#).unwrap();
        let ActivityData::Live(live) = published.data else {
            panic!("debe ser Live")
        };
        assert_eq!(live.focus_pid, Some(4242));
        assert!(live.focusable);
        let json = serde_json::to_value(&live).unwrap();
        assert!(json.get("focusPid").is_none() && json["focusable"] == true);

        assert!(activity("c", br#"{"title":"t","focusPid":0}"#).is_err());
    }

    #[test]
    fn progress_is_clamped_and_must_be_finite() {
        let activity = activity("p", br#"{"title":"t","progress":1.7}"#).unwrap();
        let ActivityData::Live(live) = activity.data else {
            panic!("debe ser Live")
        };
        assert_eq!(live.progress, Some(1.0));
    }

    #[test]
    fn bad_bodies_are_rejected() {
        for body in [
            &br#"{"title":""}"#[..],
            br#"{"subtitle":"x"}"#,
            br#"{"title":"t","color":"red"}"#,
            br#"{"title":"t","state":"exploded"}"#,
            br#"{"title":"t","icon":"https://evil"}"#,
            b"no es json",
        ] {
            assert!(
                matches!(activity("x", body), Err(ApiError::BadRequest(_))),
                "{}",
                String::from_utf8_lossy(body)
            );
        }
        let long_title = format!(r#"{{"title":"{}"}}"#, "a".repeat(81));
        assert!(activity("x", long_title.as_bytes()).is_err());
    }
}

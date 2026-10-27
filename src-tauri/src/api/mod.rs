//! API local de Live Activities: scripts, hooks y apps publican actividades en la isla.
//!
//! Seguridad (ver `docs/ADRs/0006-api-local-de-live-activities.md`):
//! - Solo escucha en `127.0.0.1`.
//! - Todas las rutas exigen `Authorization: Bearer <token>`.
//! - Rechaza cualquier petición con header `Origin`: una página web puede hacerle `fetch`
//!   a localhost, pero el navegador siempre manda `Origin` y aquí se corta.

mod request;
pub mod token;

use std::io::{Cursor, Read};
use std::thread;

use tiny_http::{Header, Request, Response, Server};

use crate::activity::ActivityHub;
use request::{ApiError, ID_PREFIX, MAX_BODY, Route};

pub const DEFAULT_PORT: u16 = 47823;

/// Arranca el servidor. Si el puerto está ocupado, la app sigue sin API en vez de
/// morir: la isla no depende de ella.
pub fn start(hub: ActivityHub, port: u16, token: String) -> std::io::Result<()> {
    let server = match Server::http(("127.0.0.1", port)) {
        Ok(server) => server,
        Err(err) => {
            eprintln!("API local desactivada: no se pudo abrir 127.0.0.1:{port}: {err}");
            return Ok(());
        }
    };
    thread::Builder::new()
        .name("live-activities-api".into())
        .spawn(move || {
            for mut request in server.incoming_requests() {
                let response = handle(&hub, &token, &mut request);
                let _ = request.respond(response);
            }
        })?;
    Ok(())
}

fn handle(hub: &ActivityHub, token: &str, request: &mut Request) -> Response<Cursor<Vec<u8>>> {
    let header = |name: &'static str| {
        request
            .headers()
            .iter()
            .find(|h| h.field.equiv(name))
            .map(|h| h.value.as_str().to_owned())
    };
    if header("Origin").is_some() {
        return error(ApiError::Forbidden);
    }
    if !request::authorized(header("Authorization").as_deref(), token) {
        return error(ApiError::Unauthorized);
    }

    match request::route(request.method().as_str(), request.url()) {
        Err(err) => error(err),
        Ok(Route::Health) => json(200, r#"{"ok":true}"#.into()),
        Ok(Route::Delete(id)) => {
            hub.remove(format!("{ID_PREFIX}{id}"));
            empty()
        }
        Ok(Route::Put(id)) => {
            // Lee un byte de más para distinguir "justo en el límite" de "se pasó".
            let mut body = Vec::new();
            let read = request
                .as_reader()
                .take(MAX_BODY as u64 + 1)
                .read_to_end(&mut body);
            if read.is_err() {
                return error(ApiError::BadRequest("no se pudo leer el cuerpo".into()));
            }
            if body.len() > MAX_BODY {
                return error(ApiError::PayloadTooLarge);
            }
            match request::activity(&id, &body) {
                Ok(activity) => {
                    hub.upsert(activity);
                    empty()
                }
                Err(err) => error(err),
            }
        }
    }
}

fn json(status: u16, body: String) -> Response<Cursor<Vec<u8>>> {
    let content_type = Header::from_bytes("Content-Type", "application/json; charset=utf-8")
        .expect("header estático válido");
    Response::from_string(body)
        .with_status_code(status)
        .with_header(content_type)
}

fn empty() -> Response<Cursor<Vec<u8>>> {
    Response::from_string(String::new()).with_status_code(204)
}

fn error(err: ApiError) -> Response<Cursor<Vec<u8>>> {
    let body = serde_json::json!({ "error": err.message() }).to_string();
    json(err.status(), body)
}

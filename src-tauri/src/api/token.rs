//! Token de la API: 32 bytes aleatorios del CSPRNG de Windows, en hex, guardados en el
//! directorio de configuración del usuario (que solo él puede leer).

use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::Path;

use windows::Win32::Security::Cryptography::{BCRYPT_USE_SYSTEM_PREFERRED_RNG, BCryptGenRandom};

const TOKEN_BYTES: usize = 32;

/// Reusa el token guardado o genera uno nuevo. Mantenerlo entre reinicios permite que
/// los scripts y hooks lo lean una sola vez.
pub fn load_or_create(path: &Path) -> io::Result<String> {
    if let Ok(saved) = fs::read_to_string(path) {
        let saved = saved.trim();
        if is_valid(saved) {
            return Ok(saved.to_owned());
        }
    }
    let token = generate()?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, &token)?;
    Ok(token)
}

fn generate() -> io::Result<String> {
    let mut bytes = [0u8; TOKEN_BYTES];
    // SAFETY: el búfer es válido y del tamaño declarado.
    unsafe { BCryptGenRandom(None, &mut bytes, BCRYPT_USE_SYSTEM_PREFERRED_RNG) }
        .ok()
        .map_err(io::Error::other)?;
    Ok(bytes.iter().fold(String::new(), |mut hex, byte| {
        let _ = write!(hex, "{byte:02x}");
        hex
    }))
}

fn is_valid(token: &str) -> bool {
    token.len() == TOKEN_BYTES * 2 && token.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir()
            .join(format!("notch-token-{}-{name}", std::process::id()))
            .join("api-token")
    }

    #[test]
    fn generates_hex_tokens_that_differ() {
        let (a, b) = (generate().unwrap(), generate().unwrap());
        assert!(is_valid(&a) && is_valid(&b));
        assert_ne!(a, b);
    }

    #[test]
    fn keeps_a_saved_token_and_replaces_a_corrupt_one() {
        let path = temp_path("persist");
        let first = load_or_create(&path).unwrap();
        assert_eq!(load_or_create(&path).unwrap(), first);

        fs::write(&path, "basura").unwrap();
        let replaced = load_or_create(&path).unwrap();
        assert!(is_valid(&replaced));
        assert_ne!(replaced, first);

        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}

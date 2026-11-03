//! Contraseñas: hash argon2id (parámetros por defecto del crate = recomendación OWASP:
//! m=19 MiB, t=2, p=1) con sal aleatoria. El cálculo corre en un hilo bloqueante para no
//! frenar el runtime async.

use std::sync::OnceLock;

use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};

use crate::error::ApiError;

pub const LONGITUD_MIN: usize = 12;
pub const LONGITUD_MAX: usize = 128;

/// Política (alineada con NIST 800-63B: longitud, sin reglas de composición).
/// TODO: verificar contra listas de contraseñas filtradas (ej. k-anonymity de HIBP).
pub fn validar_politica(password: &str) -> Result<(), String> {
    let n = password.chars().count();
    if n < LONGITUD_MIN {
        return Err(format!(
            "la contraseña debe tener al menos {LONGITUD_MIN} caracteres"
        ));
    }
    if n > LONGITUD_MAX {
        return Err(format!(
            "la contraseña no puede superar {LONGITUD_MAX} caracteres"
        ));
    }
    if password.trim().is_empty() {
        return Err("la contraseña no puede ser solo espacios".into());
    }
    Ok(())
}

pub fn hashear_sync(password: &str) -> Result<String, String> {
    let sal = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &sal)
        .map(|h| h.to_string())
        .map_err(|e| format!("error al hashear: {e}"))
}

fn verificar_sync(hash: &str, password: &str) -> bool {
    PasswordHash::new(hash)
        .map(|h| {
            Argon2::default()
                .verify_password(password.as_bytes(), &h)
                .is_ok()
        })
        .unwrap_or(false)
}

/// Hash de una contraseña aleatoria, usado cuando el email no existe para que el login
/// tarde lo mismo (evita enumerar usuarios por tiempo de respuesta).
fn hash_señuelo() -> &'static str {
    static H: OnceLock<String> = OnceLock::new();
    H.get_or_init(|| hashear_sync(&uuid::Uuid::new_v4().to_string()).expect("hash señuelo"))
}

pub async fn hashear(password: String) -> Result<String, ApiError> {
    tokio::task::spawn_blocking(move || hashear_sync(&password))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .map_err(ApiError::Internal)
}

/// Verifica `password` contra `hash`; con `None` verifica contra el señuelo y devuelve false.
pub async fn verificar(hash: Option<String>, password: String) -> Result<bool, ApiError> {
    tokio::task::spawn_blocking(move || match hash {
        Some(h) => verificar_sync(&h, &password),
        None => {
            let _ = verificar_sync(hash_señuelo(), &password);
            false
        }
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))
}

/// Precalcula el hash señuelo al arrancar (evita que el primer login falso tarde más).
pub fn precalentar() {
    let _ = hash_señuelo();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn politica() {
        assert!(validar_politica("corta").is_err());
        assert!(validar_politica(&"x".repeat(129)).is_err());
        assert!(validar_politica(&" ".repeat(20)).is_err());
        assert!(validar_politica("una frase larga cualquiera").is_ok());
        assert!(validar_politica("ñandú-ñandú!").is_ok()); // 12 caracteres (no bytes)
    }

    #[tokio::test]
    async fn hash_y_verificacion() {
        let h = hashear("contraseña correcta".into()).await.unwrap();
        assert!(h.starts_with("$argon2id$"), "{h}");
        assert!(
            verificar(Some(h.clone()), "contraseña correcta".into())
                .await
                .unwrap()
        );
        assert!(
            !verificar(Some(h.clone()), "contraseña incorrecta".into())
                .await
                .unwrap()
        );
        assert!(
            !verificar(Some("no-es-un-hash".into()), "x".into())
                .await
                .unwrap()
        );
        assert!(!verificar(None, "contraseña correcta".into()).await.unwrap());
        // Sal aleatoria: dos hashes de la misma contraseña difieren.
        assert_ne!(h, hashear("contraseña correcta".into()).await.unwrap());
    }
}

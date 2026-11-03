//! Comandos administrativos:
//!   api establecer-credenciales <usuario_id> <email>
//! La contraseña se pide por terminal sin eco (dos veces) o se lee de stdin si no es una
//! terminal (para scripts). Nunca se pasa como argumento (quedaría en el historial).

use std::io::{BufRead, IsTerminal};

use crate::passwords;

pub fn es_comando(args: &[String]) -> bool {
    args.get(1).is_some_and(|a| a == "establecer-credenciales")
}

fn leer_password() -> Result<String, String> {
    if std::io::stdin().is_terminal() {
        let p1 = rpassword::prompt_password("Contraseña: ").map_err(|e| e.to_string())?;
        let p2 = rpassword::prompt_password("Repetir contraseña: ").map_err(|e| e.to_string())?;
        if p1 != p2 {
            return Err("las contraseñas no coinciden".into());
        }
        Ok(p1)
    } else {
        let mut linea = String::new();
        std::io::stdin()
            .lock()
            .read_line(&mut linea)
            .map_err(|e| e.to_string())?;
        Ok(linea.trim_end_matches(['\r', '\n']).to_string())
    }
}

pub async fn ejecutar(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let (Some(id), Some(email)) = (args.get(2), args.get(3)) else {
        return Err("uso: api establecer-credenciales <usuario_id> <email>".into());
    };
    let usuario_id: uuid::Uuid = id.parse().map_err(|_| "usuario_id no es un UUID")?;
    if !email.contains('@') {
        return Err("email inválido".into());
    }
    let password = leer_password()?;
    passwords::validar_politica(&password)?;
    let hash = passwords::hashear_sync(&password)?;

    let url = std::env::var("DATABASE_URL").map_err(|_| "falta DATABASE_URL")?;
    let pool = sqlx::PgPool::connect(&url).await?;
    db::auth::guardar_credencial(&pool, usuario_id, email, &hash)
        .await
        .map_err(|e| format!("no se pudo guardar (¿existe el usuario en `usuarios`?): {e}"))?;
    let n = db::auth::revocar_todas(&pool, usuario_id).await?;
    println!(
        "credenciales guardadas para {} ({n} sesiones revocadas)",
        email.to_lowercase()
    );
    Ok(())
}

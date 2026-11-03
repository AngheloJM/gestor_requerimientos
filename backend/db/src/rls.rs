//! Propagación de la identidad del usuario a Postgres para que **RLS** autorice.
//!
//! Por cada request autenticado se abre una transacción y, antes de cualquier consulta:
//! ```sql
//! SET LOCAL ROLE authenticated;
//! SELECT set_config('request.jwt.claims', '{"sub":"<user_id>","role":"authenticated"}', true);
//! -- Forma antigua (variables separadas), por compatibilidad:
//! SELECT set_config('request.jwt.claim.sub', '<user_id>', true);
//! SELECT set_config('request.jwt.claim.role', 'authenticated', true);
//! ```
//! Se definen ambas formas porque según la versión de Supabase `auth.uid()` lee
//! `request.jwt.claims` y/o `request.jwt.claim.sub` (la imagen `supabase/postgres` sin el
//! servicio Auth trae la versión que lee solo `request.jwt.claim.sub`).
//! Ambos son locales a la transacción (`SET LOCAL` / `is_local = true`): al terminar
//! (commit o rollback) se descartan, así que no se filtran a otro usuario aunque la conexión
//! vuelva al pool o pase por pgbouncer/Supavisor en modo transacción.
//!
//! Requisito de la base: el usuario de `DATABASE_URL` debe poder hacer `SET ROLE authenticated`
//! (miembro de ese rol). Se verifica al arrancar con [`verificar`].

use domain::UsuarioId;
use sqlx::{PgPool, Postgres, Transaction};

/// Abre una transacción con la identidad del usuario. Devolverla al pool sin `commit`
/// equivale a rollback.
pub async fn begin_como_usuario(
    pool: &PgPool,
    usuario: UsuarioId,
) -> Result<Transaction<'static, Postgres>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL ROLE authenticated")
        .execute(&mut *tx)
        .await?;
    // JSON construido con serde (no por concatenación de strings).
    let claims = serde_json::json!({ "sub": usuario.to_string(), "role": "authenticated" });
    sqlx::query(
        "SELECT set_config('request.jwt.claims', $1, true),
                set_config('request.jwt.claim.sub', $2, true),
                set_config('request.jwt.claim.role', 'authenticated', true)",
    )
    .bind(claims.to_string())
    .bind(usuario.to_string())
    .execute(&mut *tx)
    .await?;
    Ok(tx)
}

/// Comprueba al arrancar que la conexión puede asumir el rol `authenticated`.
pub async fn verificar(pool: &PgPool) -> Result<(), String> {
    let mut tx = begin_como_usuario(pool, UsuarioId(uuid::Uuid::nil()))
        .await
        .map_err(|e| {
            format!(
                "no se pudo ejecutar SET LOCAL ROLE authenticated ({e}). \
                 El usuario de DATABASE_URL debe ser miembro del rol `authenticated`"
            )
        })?;
    let rol: String = sqlx::query_scalar("SELECT current_user::text")
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    tx.rollback().await.map_err(|e| e.to_string())?;
    if rol == "authenticated" {
        Ok(())
    } else {
        Err(format!(
            "se esperaba current_user = authenticated, se obtuvo {rol}"
        ))
    }
}

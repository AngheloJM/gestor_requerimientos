//! Utilidades compartidas por los tests de integración.
//!
//! Requieren `DATABASE_URL` apuntando a un Postgres de PRUEBAS (sqlx crea y borra una base
//! temporal por test). Ejecutar con: `cargo test -p db -- --ignored`.
//! Fixtures: `supabase_subset` (schema), `rls_test` (rol authenticated + políticas de prueba),
//! `datos` (datos ficticios).
#![allow(dead_code)]

use db::{PgPool, rls::begin_como_usuario};
use domain::UsuarioId;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub const RRHH: &str = "00000000-0000-0000-0000-aaaa00000001";
/// encargado_costos con Call center, Campo, Instalaciones.
pub const COSTOS: &str = "00000000-0000-0000-0000-aaaa00000002";
/// estandar con Soporte.
pub const ESTANDAR: &str = "00000000-0000-0000-0000-aaaa00000003";
/// estandar sin servicios asignados (es solicitante de dos solicitudes de cambio).
pub const SIN_SERVICIOS: &str = "00000000-0000-0000-0000-aaaa00000004";

pub fn uid(s: &str) -> Uuid {
    Uuid::parse_str(s).unwrap()
}

pub async fn como(pool: &PgPool, usuario: &str) -> Transaction<'static, Postgres> {
    begin_como_usuario(pool, UsuarioId(uid(usuario)))
        .await
        .unwrap()
}

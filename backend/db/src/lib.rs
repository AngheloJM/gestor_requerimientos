//! Acceso a datos (Postgres de Supabase vía SQLx).
//!
//! **Toda consulta de datos de negocio corre dentro de una transacción abierta con
//! [`rls::begin_como_usuario`]**, para que las políticas RLS de Postgres apliquen la
//! autorización. Los repositorios reciben `&mut PgConnection` (la de esa transacción),
//! nunca el pool directamente.
//!
//! Las columnas cuyo tipo exacto no está confirmado se castean en SQL (`::numeric`, `::text`,
//! `::date`, `::timestamptz`) para que la decodificación no dependa de ese detalle.

use std::fmt;

pub mod almanaque;
pub mod aprobaciones;
pub mod auth;
pub mod matriz;
pub mod requerimientos;
pub mod rls;
pub mod tablero;
pub mod usuarios;

pub use sqlx::{PgConnection, PgPool};

/// SQLSTATE `insufficient_privilege`: lo devuelve Postgres cuando RLS o los GRANT impiden
/// una operación (ej. "new row violates row-level security policy").
const SQLSTATE_PERMISO_DENEGADO: &str = "42501";

#[derive(Debug)]
pub enum DbError {
    Sqlx(sqlx::Error),
    /// La base devolvió datos que violan una regla del dominio.
    DatosInvalidos(domain::DomainError),
}

impl DbError {
    /// `true` si Postgres rechazó la operación por permisos/RLS.
    pub fn es_permiso_denegado(&self) -> bool {
        matches!(self, Self::Sqlx(sqlx::Error::Database(e))
            if e.code().as_deref() == Some(SQLSTATE_PERMISO_DENEGADO))
    }
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlx(e) => write!(f, "error de base de datos: {e}"),
            Self::DatosInvalidos(e) => write!(f, "datos inválidos en la base: {e}"),
        }
    }
}

impl std::error::Error for DbError {}

impl From<sqlx::Error> for DbError {
    fn from(e: sqlx::Error) -> Self {
        Self::Sqlx(e)
    }
}

impl From<domain::DomainError> for DbError {
    fn from(e: domain::DomainError) -> Self {
        Self::DatosInvalidos(e)
    }
}

/// Convierte una cantidad entera de la base a `u32`, rechazando negativos.
pub(crate) fn cantidad_desde_db(campo: &'static str, valor: i64) -> Result<u32, DbError> {
    u32::try_from(valor).map_err(|_| {
        DbError::DatosInvalidos(domain::DomainError::CantidadInvalida { campo, valor })
    })
}

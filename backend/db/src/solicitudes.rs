//! Solicitudes de cambio (Kanban). Reglas en la base (verificadas en producción): CHECKs,
//! trigger `validar_solicitud_cambio` y RLS.
//!
//! - Crear: usuario activo con el servicio asignado (o RR.HH.); queda `pendiente`.
//! - Resolver / Rechazar: RR.HH., o Costos si `campo = 'centro_costo'`; comentario obligatorio.
//! - Cancelar: solo quien la pidió; comentario opcional.
//! - Solo se cierran solicitudes `pendiente`; la base fija `responsable_id` y `cerrada_at`.

use domain::{CargoId, ServicioId};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::DbError;

/// `solicitudes_cambio.campo` admitidos (CHECK de la base).
pub const CAMPOS: [&str; 8] = [
    "centro_costo",
    "cliente",
    "contrato_comercial",
    "tipo_contrato",
    "modalidad",
    "dias_trabajo",
    "sueldo_basico",
    "bonos",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cierre {
    Resolver,
    Rechazar,
    Cancelar,
}

impl Cierre {
    /// Estado resultante en la base.
    pub fn estado(self) -> &'static str {
        match self {
            Self::Resolver => "resuelta",
            Self::Rechazar => "rechazada",
            Self::Cancelar => "cancelada",
        }
    }
}

#[derive(Debug)]
pub enum CerrarError {
    /// No existe, ya está cerrada o RLS no permite modificarla.
    NoDisponible,
    Db(DbError),
}

impl From<sqlx::Error> for CerrarError {
    fn from(e: sqlx::Error) -> Self {
        Self::Db(DbError::Sqlx(e))
    }
}

pub async fn cerrar(
    conn: &mut PgConnection,
    id: Uuid,
    cierre: Cierre,
    comentario: Option<&str>,
) -> Result<(), CerrarError> {
    let n = sqlx::query(
        "UPDATE solicitudes_cambio SET estado = $2, comentario_resolucion = $3 WHERE id = $1",
    )
    .bind(id)
    .bind(cierre.estado())
    .bind(comentario)
    .execute(conn)
    .await?
    .rows_affected();
    if n == 0 {
        return Err(CerrarError::NoDisponible);
    }
    Ok(())
}

pub struct NuevaSolicitud<'a> {
    pub servicio_id: ServicioId,
    /// Obligatorio para `sueldo_basico` y `bonos` (lo valida la base).
    pub cargo_id: Option<CargoId>,
    pub campo: &'a str,
    pub motivo: &'a str,
    pub descripcion: Option<&'a str>,
}

/// Crea una solicitud a nombre del usuario actual. Devuelve su id.
pub async fn crear(conn: &mut PgConnection, s: &NuevaSolicitud<'_>) -> Result<Uuid, DbError> {
    Ok(sqlx::query_scalar(
        "INSERT INTO solicitudes_cambio (servicio_id, cargo_id, campo, solicitante_id, motivo, descripcion, estado)
         VALUES ($1, $2, $3, auth.uid(), $4, $5, 'pendiente') RETURNING id",
    )
    .bind(Uuid::from(s.servicio_id))
    .bind(s.cargo_id.map(Uuid::from))
    .bind(s.campo)
    .bind(s.motivo)
    .bind(s.descripcion)
    .fetch_one(conn)
    .await?)
}

//! Días feriados (`feriados`, migración 0005) y tiempos de capacitación para los plazos de un
//! requerimiento nuevo. Lectura: cualquier usuario autenticado; alta y baja: RR.HH. (RLS).
//! Alcance actual: solo feriados nacionales.

use std::collections::HashSet;

use chrono::{DateTime, NaiveDate, Utc};
use domain::plazos::TiemposCapacitacion;
use serde::Serialize;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::DbError;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Feriado {
    pub id: Uuid,
    pub fecha: NaiveDate,
    pub nombre: String,
    pub created_at: DateTime<Utc>,
}

/// Feriados de un año, por fecha.
pub async fn del_anio(conn: &mut PgConnection, anio: i32) -> Result<Vec<Feriado>, DbError> {
    Ok(sqlx::query_as(
        "SELECT id, fecha, nombre, created_at FROM feriados
          WHERE extract(year FROM fecha) = $1 ORDER BY fecha",
    )
    .bind(anio)
    .fetch_all(conn)
    .await?)
}

/// Fechas feriadas en `[desde, hasta)`.
pub async fn entre(
    conn: &mut PgConnection,
    desde: NaiveDate,
    hasta: NaiveDate,
) -> Result<HashSet<NaiveDate>, DbError> {
    let v: Vec<NaiveDate> =
        sqlx::query_scalar("SELECT fecha FROM feriados WHERE fecha >= $1 AND fecha < $2")
            .bind(desde)
            .bind(hasta)
            .fetch_all(conn)
            .await?;
    Ok(v.into_iter().collect())
}

pub async fn crear(
    conn: &mut PgConnection,
    fecha: NaiveDate,
    nombre: &str,
) -> Result<Uuid, DbError> {
    Ok(
        sqlx::query_scalar("INSERT INTO feriados (fecha, nombre) VALUES ($1, $2) RETURNING id")
            .bind(fecha)
            .bind(nombre)
            .fetch_one(conn)
            .await?,
    )
}

/// Filas borradas (0 si no existe o RLS no deja borrarlo).
pub async fn eliminar(conn: &mut PgConnection, id: Uuid) -> Result<u64, DbError> {
    Ok(sqlx::query("DELETE FROM feriados WHERE id = $1")
        .bind(id)
        .execute(conn)
        .await?
        .rows_affected())
}

/// Tiempos del cargo en el servicio (activo) y el general del servicio. Si el cargo no está
/// en el servicio, solo vuelve el general; si el servicio no es visible o está inactivo, nada.
pub async fn tiempos_capacitacion(
    conn: &mut PgConnection,
    servicio_id: Uuid,
    cargo_id: Uuid,
) -> Result<TiemposCapacitacion, DbError> {
    let fila: Option<(Option<i32>, Option<i32>, Option<i32>)> = sqlx::query_as(
        "SELECT sc.tiempo_capacitacion_dias::int4, sc.tiempo_capacitacion_ajustada_dias::int4,
                s.tiempo_capacitacion_dias::int4
           FROM servicios s
           LEFT JOIN servicio_cargos sc ON sc.servicio_id = s.id AND sc.cargo_id = $2
          WHERE s.id = $1 AND s.activo IS TRUE",
    )
    .bind(servicio_id)
    .bind(cargo_id)
    .fetch_optional(conn)
    .await?;
    Ok(fila.map_or_else(TiemposCapacitacion::default, |(c, a, g)| {
        TiemposCapacitacion {
            capacitacion_cargo: c,
            ajustada_cargo: a,
            general_servicio: g,
        }
    }))
}

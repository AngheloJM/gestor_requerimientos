//! Listado de requerimientos visibles para el usuario ("Mis requerimientos").
//!
//! Autorización: RLS (`requerimientos_select` en producción: solicitante, RR.HH., servicio
//! asignado o inmediato superior del solicitante). Solo servicios activos (o sin servicio).

use chrono::{DateTime, NaiveDate, Utc};
use domain::{RequerimientoResumen, UsuarioId};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::{DbError, cantidad_desde_db};

/// $1 = solicitante opcional (NULL = todos los visibles).
const QUERY: &str = r#"
SELECT
    r.id,
    r.numero::int8                 AS numero,
    r.folio::text                  AS folio,
    r.solicitante_id,
    r.servicio_id,
    s.nombre::text                 AS servicio_nombre,
    r.cargo_id,
    c.nombre::text                 AS cargo_nombre,
    r.tipo::text                   AS tipo,
    r.motivo::text                 AS motivo,
    r.cantidad_personas::int8      AS cantidad,
    r.estado::text                 AS estado,
    r.fecha_inicio::date           AS fecha_inicio,
    r.fecha_inicio_operacion::date AS fecha_inicio_operacion,
    r.created_at::timestamptz      AS created_at
FROM requerimientos r
LEFT JOIN servicios s ON s.id = r.servicio_id
LEFT JOIN cargos    c ON c.id = r.cargo_id
WHERE ($1::uuid IS NULL OR r.solicitante_id = $1)
  AND (r.servicio_id IS NULL OR s.activo IS TRUE)
ORDER BY r.created_at DESC, r.numero DESC
"#;

#[derive(FromRow)]
struct Row {
    id: Uuid,
    numero: i64,
    folio: Option<String>,
    solicitante_id: Option<Uuid>,
    servicio_id: Option<Uuid>,
    servicio_nombre: Option<String>,
    cargo_id: Uuid,
    cargo_nombre: Option<String>,
    tipo: Option<String>,
    motivo: Option<String>,
    cantidad: Option<i64>,
    estado: Option<String>,
    fecha_inicio: NaiveDate,
    fecha_inicio_operacion: Option<NaiveDate>,
    created_at: DateTime<Utc>,
}

/// Requerimientos visibles, del más reciente al más antiguo. `solicitante = Some(u)` limita
/// a los que pidió `u`.
pub async fn listar(
    conn: &mut PgConnection,
    solicitante: Option<UsuarioId>,
) -> Result<Vec<RequerimientoResumen>, DbError> {
    let rows: Vec<Row> = sqlx::query_as(QUERY)
        .bind(solicitante.map(Uuid::from))
        .fetch_all(conn)
        .await?;
    rows.into_iter()
        .map(|r| {
            Ok(RequerimientoResumen {
                id: r.id.into(),
                numero: r.numero,
                folio: r.folio,
                solicitante_id: r.solicitante_id.map(Into::into),
                servicio_id: r.servicio_id.map(Into::into),
                servicio_nombre: r.servicio_nombre,
                cargo_id: r.cargo_id.into(),
                cargo_nombre: r.cargo_nombre,
                tipo: r.tipo,
                motivo: r.motivo,
                cantidad: r
                    .cantidad
                    .map(|c| cantidad_desde_db("requerimientos.cantidad_personas", c))
                    .transpose()?,
                estado: r.estado,
                fecha_inicio: r.fecha_inicio,
                fecha_inicio_operacion: r.fecha_inicio_operacion,
                created_at: r.created_at,
            })
        })
        .collect()
}

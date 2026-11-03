//! Tablero: (1) completitud de servicios activos para los indicadores y
//! (2) `solicitudes_cambio` para el Kanban. Visibilidad: RLS.

use chrono::{DateTime, NaiveDate, Utc};
use domain::{ContratoComercial, EstadoContrato, EstadoServicio, SolicitudCambio};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::{DbError, cantidad_desde_db};

const QUERY_COMPLETITUD: &str = r#"
SELECT
    s.id                        AS servicio_id,
    s.nombre::text              AS servicio_nombre,
    cl.nombre::text             AS cliente_nombre,
    s.contrato_comercial_id,
    k.id                        AS contrato_id,
    k.codigo::text              AS contrato_codigo,
    k.nombre::text              AS contrato_nombre,
    k.cliente_id                AS contrato_cliente_id,
    k.fecha_inicio::date        AS contrato_fecha_inicio,
    k.fecha_fin::date           AS contrato_fecha_fin,
    k.duracion_valor::text      AS contrato_duracion_valor,
    k.duracion_unidad::text     AS contrato_duracion_unidad,
    (SELECT count(*) FROM servicio_cargos sc WHERE sc.servicio_id = s.id) AS cantidad_cargos
FROM servicios s
LEFT JOIN clientes             cl ON cl.id = s.cliente_id
LEFT JOIN contratos_comerciales k ON k.id  = s.contrato_comercial_id
WHERE s.activo = true
ORDER BY s.nombre, s.id
"#;

/// TODO(negocio): ¿el Kanban muestra solicitudes cerradas (`cerrada_at` no nulo)? Hoy se
/// devuelven todas; el frontend puede filtrar por `estado`/`cerrada_at`.
const QUERY_SOLICITUDES: &str = r#"
SELECT
    sc.id, sc.servicio_id, s.nombre::text AS servicio_nombre,
    sc.cargo_id, c.nombre::text AS cargo_nombre,
    sc.campo::text AS campo, sc.solicitante_id, sc.motivo::text AS motivo,
    sc.descripcion::text AS descripcion, sc.estado::text AS estado, sc.responsable_id,
    sc.comentario_resolucion::text AS comentario_resolucion,
    sc.created_at::timestamptz AS created_at, sc.updated_at::timestamptz AS updated_at,
    sc.cerrada_at::timestamptz AS cerrada_at
FROM solicitudes_cambio sc
-- LEFT JOIN: el solicitante puede ver su solicitud (RLS) aunque no vea el servicio.
-- Se excluyen solo las de servicios que se sabe inactivos.
LEFT JOIN servicios s ON s.id = sc.servicio_id
LEFT JOIN cargos    c ON c.id = sc.cargo_id
WHERE s.id IS NULL OR s.activo = true
ORDER BY sc.created_at, sc.id
"#;

#[derive(FromRow)]
struct CompletitudRow {
    servicio_id: Uuid,
    servicio_nombre: String,
    cliente_nombre: Option<String>,
    contrato_comercial_id: Option<Uuid>,
    contrato_id: Option<Uuid>,
    contrato_codigo: Option<String>,
    contrato_nombre: Option<String>,
    contrato_cliente_id: Option<Uuid>,
    contrato_fecha_inicio: Option<NaiveDate>,
    contrato_fecha_fin: Option<NaiveDate>,
    contrato_duracion_valor: Option<String>,
    contrato_duracion_unidad: Option<String>,
    cantidad_cargos: i64,
}

pub async fn estado_servicios(conn: &mut PgConnection) -> Result<Vec<EstadoServicio>, DbError> {
    let rows: Vec<CompletitudRow> = sqlx::query_as(QUERY_COMPLETITUD).fetch_all(conn).await?;
    rows.into_iter()
        .map(|r| {
            let contrato = match (
                r.contrato_comercial_id,
                r.contrato_id,
                r.contrato_fecha_inicio,
            ) {
                (None, _, _) => EstadoContrato::SinContrato,
                // Referenciado pero no visible (RLS) o sin fecha de inicio legible.
                (Some(_), None, _) | (Some(_), Some(_), None) => EstadoContrato::NoVisible,
                (Some(_), Some(id), Some(inicio)) => EstadoContrato::Con(ContratoComercial::new(
                    id.into(),
                    r.contrato_codigo.unwrap_or_default(),
                    r.contrato_nombre.unwrap_or_default(),
                    r.contrato_cliente_id.map(Into::into),
                    inicio,
                    r.contrato_fecha_fin,
                    r.contrato_duracion_valor,
                    r.contrato_duracion_unidad,
                )?),
            };
            Ok(EstadoServicio {
                servicio_id: r.servicio_id.into(),
                servicio_nombre: r.servicio_nombre,
                cliente_nombre: r.cliente_nombre,
                contrato,
                cantidad_cargos: cantidad_desde_db("servicio_cargos", r.cantidad_cargos)?,
            })
        })
        .collect()
}

#[derive(FromRow)]
struct SolicitudRow {
    id: Uuid,
    servicio_id: Uuid,
    servicio_nombre: Option<String>,
    cargo_id: Option<Uuid>,
    cargo_nombre: Option<String>,
    campo: String,
    solicitante_id: Option<Uuid>,
    motivo: Option<String>,
    descripcion: Option<String>,
    estado: Option<String>,
    responsable_id: Option<Uuid>,
    comentario_resolucion: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
    cerrada_at: Option<DateTime<Utc>>,
}

pub async fn solicitudes_cambio(conn: &mut PgConnection) -> Result<Vec<SolicitudCambio>, DbError> {
    let rows: Vec<SolicitudRow> = sqlx::query_as(QUERY_SOLICITUDES).fetch_all(conn).await?;
    Ok(rows
        .into_iter()
        .map(|r| SolicitudCambio {
            id: r.id.into(),
            servicio_id: r.servicio_id.into(),
            servicio_nombre: r.servicio_nombre,
            cargo_id: r.cargo_id.map(Into::into),
            cargo_nombre: r.cargo_nombre,
            campo: r.campo,
            solicitante_id: r.solicitante_id.map(Into::into),
            motivo: r.motivo,
            descripcion: r.descripcion,
            estado: r.estado,
            responsable_id: r.responsable_id.map(Into::into),
            comentario_resolucion: r.comentario_resolucion,
            created_at: r.created_at,
            updated_at: r.updated_at,
            cerrada_at: r.cerrada_at,
        })
        .collect())
}

//! Aprobaciones sobre `requerimientos` (fecha_inicio, fecha_fin editable, cantidad_personas)
//! con trazabilidad en `requerimiento_historial` (tabla existente; no se crea ninguna).
//!
//! Autorización: RLS. Si el usuario no puede ver el requerimiento → `NoEncontrado`; si lo ve
//! pero RLS no le permite modificarlo o escribir el historial → `SinPermiso`.
//!
//! TODO(verificar en la base): si ya existe un trigger que escriba `requerimiento_historial`
//! al actualizar `requerimientos`, el INSERT explícito de aquí duplicaría la fila.
//! TODO(verificar): formato con que Lovable guarda fechas en `valor_anterior`/`valor_nuevo`
//! (aquí: ISO `YYYY-MM-DD`).

use chrono::{DateTime, NaiveDate, Utc};
use domain::{
    Aprobacion, DomainError, EntradaHistorial, Modificacion, RequerimientoId, UsuarioId,
    aprobaciones::validar_fecha_fin,
};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::{DbError, cantidad_desde_db};

/// Nombre de la columna editada, registrado en `requerimiento_historial.campo`.
const CAMPO_FECHA_FIN: &str = "fecha_fin";

/// $1 = id opcional (NULL = todos). Solo servicios activos (o requerimientos sin servicio).
const QUERY_APROBACIONES: &str = r#"
SELECT
    r.id,
    r.numero::int8                 AS numero,
    r.folio::text                  AS folio,
    r.tipo::text                   AS tipo,
    r.estado::text                 AS estado,
    r.servicio_id,
    s.nombre::text                 AS servicio_nombre,
    r.cargo_id,
    c.nombre::text                 AS cargo_nombre,
    r.cantidad_personas::int8      AS cantidad,
    r.fecha_inicio::date           AS fecha_inicio,
    r.fecha_fin_tentativa::date    AS fecha_fin_tentativa,
    r.fecha_fin::date              AS fecha_fin,
    h.usuario_id                   AS modificado_por,
    h.created_at                   AS modificado_en
FROM requerimientos r
LEFT JOIN servicios s ON s.id = r.servicio_id
LEFT JOIN cargos    c ON c.id = r.cargo_id
LEFT JOIN LATERAL (
    SELECT x.usuario_id, x.created_at::timestamptz AS created_at
    FROM requerimiento_historial x
    WHERE x.requerimiento_id = r.id AND x.campo = 'fecha_fin'
    ORDER BY x.created_at DESC, x.id DESC
    LIMIT 1
) h ON true
WHERE ($1::uuid IS NULL OR r.id = $1)
  AND (r.servicio_id IS NULL OR s.activo IS TRUE)
ORDER BY r.fecha_inicio, r.numero
"#;

const QUERY_HISTORIAL: &str = r#"
SELECT h.id::text AS id, h.campo::text AS campo,
       h.valor_anterior::text AS valor_anterior, h.valor_nuevo::text AS valor_nuevo,
       h.usuario_id, h.created_at::timestamptz AS created_at,
       h.glosa::text AS glosa, h.grupo_edicion::text AS grupo_edicion,
       h.categoria_desvio::text AS categoria_desvio, h.detalle_desvio::text AS detalle_desvio
FROM requerimiento_historial h
WHERE h.requerimiento_id = $1
ORDER BY h.created_at, h.id
"#;

#[derive(FromRow)]
struct AprobacionRow {
    id: Uuid,
    numero: i64,
    folio: Option<String>,
    tipo: Option<String>,
    estado: Option<String>,
    servicio_id: Option<Uuid>,
    servicio_nombre: Option<String>,
    cargo_id: Uuid,
    cargo_nombre: Option<String>,
    cantidad: Option<i64>,
    fecha_inicio: NaiveDate,
    fecha_fin_tentativa: Option<NaiveDate>,
    fecha_fin: Option<NaiveDate>,
    modificado_por: Option<Uuid>,
    modificado_en: Option<DateTime<Utc>>,
}

impl TryFrom<AprobacionRow> for Aprobacion {
    type Error = DbError;

    fn try_from(r: AprobacionRow) -> Result<Self, DbError> {
        Ok(Aprobacion {
            id: r.id.into(),
            numero: r.numero,
            folio: r.folio,
            tipo: r.tipo,
            estado: r.estado,
            servicio_id: r.servicio_id.map(Into::into),
            servicio_nombre: r.servicio_nombre,
            cargo_id: r.cargo_id.into(),
            cargo_nombre: r.cargo_nombre,
            cantidad: r
                .cantidad
                .map(|c| cantidad_desde_db("requerimientos.cantidad_personas", c))
                .transpose()?,
            fecha_inicio: r.fecha_inicio,
            fecha_fin_tentativa: r.fecha_fin_tentativa,
            fecha_fin: r.fecha_fin,
            ultima_modificacion: r.modificado_en.map(|fecha| Modificacion {
                usuario_id: r.modificado_por.map(Into::into),
                fecha,
            }),
        })
    }
}

#[derive(FromRow)]
struct HistorialRow {
    id: String,
    campo: String,
    valor_anterior: Option<String>,
    valor_nuevo: Option<String>,
    usuario_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    glosa: Option<String>,
    grupo_edicion: Option<String>,
    categoria_desvio: Option<String>,
    detalle_desvio: Option<String>,
}

pub async fn listar(conn: &mut PgConnection) -> Result<Vec<Aprobacion>, DbError> {
    let rows: Vec<AprobacionRow> = sqlx::query_as(QUERY_APROBACIONES)
        .bind(None::<Uuid>)
        .fetch_all(conn)
        .await?;
    rows.into_iter().map(Aprobacion::try_from).collect()
}

pub async fn obtener(
    conn: &mut PgConnection,
    id: RequerimientoId,
) -> Result<Option<Aprobacion>, DbError> {
    let row: Option<AprobacionRow> = sqlx::query_as(QUERY_APROBACIONES)
        .bind(Uuid::from(id))
        .fetch_optional(conn)
        .await?;
    row.map(Aprobacion::try_from).transpose()
}

/// Historial completo del requerimiento (todas las filas, por `created_at`).
/// `None` si el requerimiento no existe o no es visible para el usuario.
pub async fn historial(
    conn: &mut PgConnection,
    id: RequerimientoId,
) -> Result<Option<Vec<EntradaHistorial>>, DbError> {
    let visible: Option<i32> = sqlx::query_scalar("SELECT 1 FROM requerimientos WHERE id = $1")
        .bind(Uuid::from(id))
        .fetch_optional(&mut *conn)
        .await?;
    if visible.is_none() {
        return Ok(None);
    }
    let rows: Vec<HistorialRow> = sqlx::query_as(QUERY_HISTORIAL)
        .bind(Uuid::from(id))
        .fetch_all(conn)
        .await?;
    Ok(Some(
        rows.into_iter()
            .map(|h| EntradaHistorial {
                id: h.id,
                campo: h.campo,
                valor_anterior: h.valor_anterior,
                valor_nuevo: h.valor_nuevo,
                usuario_id: h.usuario_id.map(Into::into),
                created_at: h.created_at,
                glosa: h.glosa,
                grupo_edicion: h.grupo_edicion,
                categoria_desvio: h.categoria_desvio,
                detalle_desvio: h.detalle_desvio,
            })
            .collect(),
    ))
}

#[derive(Debug)]
pub enum ActualizarError {
    /// No existe o RLS no permite verlo.
    NoEncontrado,
    /// Visible pero RLS no permite modificarlo / registrar el historial.
    SinPermiso,
    Validacion(DomainError),
    Db(DbError),
}

impl From<sqlx::Error> for ActualizarError {
    fn from(e: sqlx::Error) -> Self {
        let e = DbError::Sqlx(e);
        if e.es_permiso_denegado() {
            Self::SinPermiso
        } else {
            Self::Db(e)
        }
    }
}

impl From<DbError> for ActualizarError {
    fn from(e: DbError) -> Self {
        if e.es_permiso_denegado() {
            Self::SinPermiso
        } else {
            Self::Db(e)
        }
    }
}

/// Edita `fecha_fin` y registra la fila en `requerimiento_historial`, dentro de la
/// transacción del request (el llamador hace commit). Bloquea la fila para serializar
/// ediciones concurrentes. Si el valor no cambia, no escribe nada.
pub async fn actualizar_fecha_fin(
    conn: &mut PgConnection,
    id: RequerimientoId,
    nueva: Option<NaiveDate>,
    usuario: UsuarioId,
    glosa: Option<&str>,
) -> Result<Aprobacion, ActualizarError> {
    let uid = Uuid::from(id);

    // 1. ¿Lo puede ver? (política SELECT)
    let visible: Option<(NaiveDate, Option<NaiveDate>)> = sqlx::query_as(
        "SELECT fecha_inicio::date, fecha_fin::date FROM requerimientos WHERE id = $1",
    )
    .bind(uid)
    .fetch_optional(&mut *conn)
    .await?;
    if visible.is_none() {
        return Err(ActualizarError::NoEncontrado);
    }

    // 2. Bloqueo: FOR UPDATE solo devuelve filas que además pasan las políticas UPDATE.
    let bloqueada: Option<(NaiveDate, Option<NaiveDate>)> = sqlx::query_as(
        "SELECT fecha_inicio::date, fecha_fin::date FROM requerimientos WHERE id = $1 FOR UPDATE",
    )
    .bind(uid)
    .fetch_optional(&mut *conn)
    .await?;
    let Some((fecha_inicio, anterior)) = bloqueada else {
        return Err(ActualizarError::SinPermiso);
    };

    validar_fecha_fin(fecha_inicio, nueva).map_err(ActualizarError::Validacion)?;

    if anterior != nueva {
        let res = sqlx::query(
            "UPDATE requerimientos SET fecha_fin = $2, updated_at = now() WHERE id = $1",
        )
        .bind(uid)
        .bind(nueva)
        .execute(&mut *conn)
        .await?;
        if res.rows_affected() != 1 {
            return Err(ActualizarError::SinPermiso);
        }

        sqlx::query(
            "INSERT INTO requerimiento_historial \
             (requerimiento_id, campo, valor_anterior, valor_nuevo, usuario_id, glosa) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(uid)
        .bind(CAMPO_FECHA_FIN)
        .bind(anterior.map(|f| f.to_string()))
        .bind(nueva.map(|f| f.to_string()))
        .bind(Uuid::from(usuario))
        .bind(glosa)
        .execute(&mut *conn)
        .await?;
    }

    obtener(conn, id)
        .await?
        .ok_or(ActualizarError::NoEncontrado)
}

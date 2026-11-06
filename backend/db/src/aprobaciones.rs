//! Aprobaciones sobre `requerimientos` (fecha_inicio, fecha_fin editable, cantidad_personas)
//! con trazabilidad en `requerimiento_historial`.
//!
//! La edición se delega a la función de producción `editar_requerimiento_con_historial`
//! (verificada en la base real): exige glosa (3–2000), valida la categoría de desvío, controla
//! la concurrencia con `updated_at` y los triggers `proteger_campos_requerimiento` /
//! `p2_auditar_requerimiento` escriben el historial (una fila por campo cambiado).
//! Aquí NO se inserta en el historial: se duplicaría.
//!
//! Autorización: RLS + `p2_acceso`. Si el usuario no puede ver el requerimiento →
//! `NoEncontrado`; si lo ve pero no puede editarlo → `SinPermiso`.

use chrono::{DateTime, NaiveDate, Utc};
use domain::{
    Aprobacion, DomainError, EntradaHistorial, Modificacion, RequerimientoId,
    aprobaciones::validar_fecha_fin,
};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::{DbError, cantidad_desde_db};

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
    r.fecha_inicio_operacion::date AS fecha_inicio_operacion,
    r.fecha_inicio_capacitacion::date AS fecha_inicio_capacitacion,
    r.motivo::text                 AS motivo,
    u.nombre::text                 AS solicitante_nombre,
    r.updated_at::timestamptz      AS version,
    h.usuario_id                   AS modificado_por,
    h.created_at                   AS modificado_en
FROM requerimientos r
LEFT JOIN servicios s ON s.id = r.servicio_id
LEFT JOIN cargos    c ON c.id = r.cargo_id
LEFT JOIN usuarios  u ON u.id = r.solicitante_id
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
    fecha_inicio_operacion: Option<NaiveDate>,
    fecha_inicio_capacitacion: Option<NaiveDate>,
    motivo: Option<String>,
    solicitante_nombre: Option<String>,
    modificado_por: Option<Uuid>,
    modificado_en: Option<DateTime<Utc>>,
    version: DateTime<Utc>,
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
            fecha_inicio_operacion: r.fecha_inicio_operacion,
            fecha_inicio_capacitacion: r.fecha_inicio_capacitacion,
            motivo: r.motivo,
            solicitante_nombre: r.solicitante_nombre,
            ultima_modificacion: r.modificado_en.map(|fecha| Modificacion {
                usuario_id: r.modificado_por.map(Into::into),
                fecha,
            }),
            version: r.version,
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
    /// Visible pero sin acceso de edición (`p2_acceso`) o RLS lo rechazó.
    SinPermiso,
    /// El requerimiento cambió desde que el cliente lo leyó (`version` desactualizada).
    Conflicto,
    Validacion(DomainError),
    /// Regla de negocio rechazada por la base (mensaje de `RAISE EXCEPTION`, apto para el usuario).
    Regla(String),
    Db(DbError),
}

impl From<sqlx::Error> for ActualizarError {
    fn from(e: sqlx::Error) -> Self {
        DbError::Sqlx(e).into()
    }
}

impl From<DbError> for ActualizarError {
    fn from(e: DbError) -> Self {
        if e.es_permiso_denegado() {
            return Self::SinPermiso;
        }
        if let Some(msg) = e.regla_de_negocio() {
            return if msg.starts_with("Sin acceso") {
                Self::SinPermiso
            } else if msg.starts_with("El requerimiento cambió") {
                Self::Conflicto
            } else {
                Self::Regla(msg.to_owned())
            };
        }
        Self::Db(e)
    }
}

/// Datos de una edición (vía `editar_requerimiento_con_historial`). Al menos un campo.
#[derive(Debug, Clone, Copy)]
pub struct EdicionRequerimiento<'a> {
    /// `Some(None)` = quitar la fecha de fin; `None` = no tocarla.
    pub fecha_fin: Option<Option<NaiveDate>>,
    /// Nuevo `estado` (aprobar / rechazar…). Solo RR.HH. (lo valida la base).
    pub estado: Option<&'a str>,
    /// Nueva **cantidad actual** (`cantidad_personas`). La original queda intacta en
    /// `requerimiento_referencia`. La base exige ≥ 1 y ≥ lo ya aceptado en entregas.
    pub cantidad: Option<i32>,
    /// `updated_at` que el cliente leyó (`Aprobacion::version`).
    pub version: DateTime<Utc>,
    /// Obligatoria (3–2000 caracteres; lo valida la base).
    pub glosa: &'a str,
    /// TODO(negocio): cuándo corresponde informar un desvío. Valores admitidos por la base:
    /// falla_capacitacion, rechazo_operaciones, ratio_reclutamiento, cambio_cliente.
    pub categoria_desvio: Option<&'a str>,
    pub detalle_desvio: Option<&'a str>,
}

const QUERY_EDITAR: &str = r#"
SELECT public.editar_requerimiento_con_historial(
    $1,
    CASE WHEN $2 THEN jsonb_build_object('fecha_fin', $3::text) ELSE '{}'::jsonb END
    || CASE WHEN $4::text IS NOT NULL THEN jsonb_build_object('estado', $4::text) ELSE '{}'::jsonb END
    || CASE WHEN $9::int4 IS NOT NULL THEN jsonb_build_object('cantidad_personas', $9::int4) ELSE '{}'::jsonb END,
    $5, $6, $7, $8)
"#;

/// Aplica la edición dentro de la transacción del request (el llamador hace commit). Los
/// campos que no cambian se omiten; si ninguno cambia no se escribe nada (la base rechazaría
/// "No hay cambios para guardar").
/// (fecha_inicio, fecha_fin, estado, cantidad_personas) actuales del requerimiento.
type EstadoActual = (NaiveDate, Option<NaiveDate>, Option<String>, Option<i32>);

pub async fn actualizar(
    conn: &mut PgConnection,
    id: RequerimientoId,
    e: EdicionRequerimiento<'_>,
) -> Result<Aprobacion, ActualizarError> {
    let uid = Uuid::from(id);

    let visible: Option<EstadoActual> = sqlx::query_as(
        "SELECT fecha_inicio::date, fecha_fin::date, estado::text, cantidad_personas::int4
         FROM requerimientos WHERE id = $1",
    )
    .bind(uid)
    .fetch_optional(&mut *conn)
    .await?;
    let Some((fecha_inicio, fin_actual, estado_actual, cantidad_actual)) = visible else {
        return Err(ActualizarError::NoEncontrado);
    };

    if let Some(nueva) = e.fecha_fin {
        validar_fecha_fin(fecha_inicio, nueva).map_err(ActualizarError::Validacion)?;
    }
    let fecha_fin = e.fecha_fin.filter(|f| *f != fin_actual);
    let estado = e.estado.filter(|s| Some(*s) != estado_actual.as_deref());
    let cantidad = e.cantidad.filter(|c| Some(*c) != cantidad_actual);

    if fecha_fin.is_some() || estado.is_some() || cantidad.is_some() {
        sqlx::query(QUERY_EDITAR)
            .bind(uid)
            .bind(fecha_fin.is_some())
            .bind(
                fecha_fin
                    .flatten()
                    .map(|f| f.format("%Y-%m-%d").to_string()),
            )
            .bind(estado)
            .bind(e.version)
            .bind(e.glosa)
            .bind(e.categoria_desvio)
            .bind(e.detalle_desvio)
            .bind(cantidad)
            .execute(&mut *conn)
            .await?;
    }

    obtener(conn, id)
        .await?
        .ok_or(ActualizarError::NoEncontrado)
}

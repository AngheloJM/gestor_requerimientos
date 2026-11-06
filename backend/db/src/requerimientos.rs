//! Listado de requerimientos visibles para el usuario ("Mis requerimientos").
//!
//! Autorización: RLS (`requerimientos_select` en producción: solicitante, RR.HH., servicio
//! asignado o inmediato superior del solicitante). Solo servicios activos (o sin servicio).

use chrono::{DateTime, NaiveDate, Utc};
use domain::{
    CargoOpcion, Decimal, NuevoRequerimiento, OpcionesRequerimiento, RequerimientoResumen,
    ServicioOpcion, UsuarioId,
};
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

// ---------------------------------------------------------------------------------------
// Creación ("Nuevo requerimiento")
// ---------------------------------------------------------------------------------------

/// Servicios activos con sus cargos (de `servicio_cargos`) donde el usuario puede crear.
/// Replica, solo para armar la lista, la regla de `crear_requerimiento_con_snapshot`
/// (RR.HH.: cualquier servicio activo; resto: servicios asignados). La autoridad final es la
/// función de la base, que vuelve a validarlo al crear.
const QUERY_OPCIONES_SERVICIOS: &str = r#"
SELECT s.id AS servicio_id, s.nombre::text AS servicio_nombre,
       s.tiempo_capacitacion_dias::int4 AS tiempo_capacitacion_dias,
       c.id AS cargo_id, c.nombre::text AS cargo_nombre, c.tipo::text AS cargo_tipo,
       sc.cantidad_mujeres::int4 AS cantidad_mujeres, sc.cantidad_hombres::int4 AS cantidad_hombres
FROM servicios s
JOIN servicio_cargos sc ON sc.servicio_id = s.id
JOIN cargos c ON c.id = sc.cargo_id
WHERE s.activo IS TRUE
  AND (public.is_rrhh_admin(auth.uid())
       OR EXISTS (SELECT 1 FROM usuario_servicios us
                  WHERE us.usuario_id = auth.uid() AND us.servicio_id = s.id))
ORDER BY s.nombre, s.id, c.nombre
"#;

const QUERY_CARGOS_STAFF: &str = r#"
SELECT c.id AS cargo_id, c.nombre::text AS cargo_nombre, c.tipo::text AS cargo_tipo
FROM cargos c WHERE c.tipo = 'administrativo_staff' ORDER BY c.nombre
"#;

/// Llama a la función de producción. Las claves del JSON son las que ella acepta; los nulos
/// se omiten. Los montos viajan como `numeric` (sin pasar por float).
const QUERY_CREAR: &str = r#"
SELECT f.folio::text FROM public.crear_requerimiento_con_snapshot(
    jsonb_strip_nulls(jsonb_build_object(
        'tipo', $1::text,
        'servicioId', $2::uuid,
        'cargoId', $3::uuid,
        'cantidadPersonas', $4::int4,
        'motivo', $5::text,
        'fechaInicioCapacitacion', to_char($6::date, 'YYYY-MM-DD'),
        'fechaInicioOperacion', to_char($7::date, 'YYYY-MM-DD'),
        'fechaFin', to_char($8::date, 'YYYY-MM-DD'),
        'comentarios', $9::text))
    || jsonb_build_object('bonosExtra', (
        SELECT coalesce(jsonb_agg(jsonb_build_object(
                   'tipo', 'extra', 'nombre', b.nombre, 'monto', b.monto, 'motivo', b.motivo)
                   ORDER BY b.i), '[]'::jsonb)
        FROM unnest($10::text[], $11::numeric[], $12::text[])
             WITH ORDINALITY AS b(nombre, monto, motivo, i)))
) AS f
"#;

#[derive(FromRow)]
struct OpcionRow {
    servicio_id: Uuid,
    servicio_nombre: String,
    tiempo_capacitacion_dias: Option<i32>,
    cargo_id: Uuid,
    cargo_nombre: String,
    cargo_tipo: Option<String>,
    cantidad_mujeres: Option<i32>,
    cantidad_hombres: Option<i32>,
}

#[derive(FromRow)]
struct CargoRow {
    cargo_id: Uuid,
    cargo_nombre: String,
    cargo_tipo: Option<String>,
}

impl From<CargoRow> for CargoOpcion {
    fn from(c: CargoRow) -> Self {
        CargoOpcion {
            id: c.cargo_id.into(),
            nombre: c.cargo_nombre,
            tipo: c.cargo_tipo,
            cantidad_mujeres: None,
            cantidad_hombres: None,
        }
    }
}

pub async fn opciones(conn: &mut PgConnection) -> Result<OpcionesRequerimiento, DbError> {
    let filas: Vec<OpcionRow> = sqlx::query_as(QUERY_OPCIONES_SERVICIOS)
        .fetch_all(&mut *conn)
        .await?;
    let mut servicios: Vec<ServicioOpcion> = Vec::new();
    for f in filas {
        let cargo = CargoOpcion {
            id: f.cargo_id.into(),
            nombre: f.cargo_nombre,
            tipo: f.cargo_tipo,
            cantidad_mujeres: f.cantidad_mujeres,
            cantidad_hombres: f.cantidad_hombres,
        };
        match servicios.last_mut() {
            Some(s) if Uuid::from(s.id) == f.servicio_id => s.cargos.push(cargo),
            _ => servicios.push(ServicioOpcion {
                id: f.servicio_id.into(),
                nombre: f.servicio_nombre,
                tiempo_capacitacion_dias: f.tiempo_capacitacion_dias,
                cargos: vec![cargo],
            }),
        }
    }
    let staff: Vec<CargoRow> = sqlx::query_as(QUERY_CARGOS_STAFF).fetch_all(conn).await?;
    Ok(OpcionesRequerimiento {
        servicios,
        cargos_staff: staff.into_iter().map(Into::into).collect(),
    })
}

/// Crea el requerimiento (estado `pendiente`, solicitante = usuario actual) y devuelve su
/// folio. Las reglas de negocio rechazadas vuelven como `DbError::regla_de_negocio`.
pub async fn crear(conn: &mut PgConnection, n: &NuevoRequerimiento) -> Result<String, DbError> {
    let nombres: Vec<&str> = n.bonos_extra.iter().map(|b| b.nombre.as_str()).collect();
    let montos: Vec<Decimal> = n.bonos_extra.iter().map(|b| b.monto).collect();
    let motivos: Vec<&str> = n.bonos_extra.iter().map(|b| b.motivo.as_str()).collect();
    let cantidad = i32::try_from(n.cantidad).unwrap_or(i32::MAX);
    let folio: String = sqlx::query_scalar(QUERY_CREAR)
        .bind(&n.tipo)
        .bind(n.servicio_id.map(Uuid::from))
        .bind(Uuid::from(n.cargo_id))
        .bind(cantidad)
        .bind(&n.motivo)
        .bind(n.fecha_inicio_capacitacion)
        .bind(n.fecha_inicio_operacion)
        .bind(n.fecha_fin)
        .bind(n.comentarios.as_deref())
        .bind(&nombres)
        .bind(&montos)
        .bind(&motivos)
        .fetch_one(conn)
        .await?;
    Ok(folio)
}

// ---------------------------------------------------------------------------------------
// Detalle de un requerimiento (pantalla de solo lectura)
// ---------------------------------------------------------------------------------------

/// Visibilidad: la decide RLS (`requerimientos_select`). Mismo criterio que el listado: solo
/// servicios activos (o sin servicio). Los datos del servicio son los **actuales**; sueldo,
/// bonos y tiempo de capacitación son los congelados al crear (`*_snap`).
const QUERY_DETALLE: &str = r#"
SELECT
    r.id, r.numero::int8 AS numero, r.folio::text AS folio, r.tipo::text AS tipo,
    r.estado::text AS estado, r.motivo::text AS motivo, r.cantidad_personas::int4 AS cantidad,
    r.fecha_inicio::date AS fecha_inicio, r.fecha_fin_tentativa::date AS fecha_fin_tentativa,
    r.fecha_inicio_capacitacion::date AS fecha_inicio_capacitacion,
    r.fecha_inicio_operacion::date AS fecha_inicio_operacion,
    r.comentarios_adicionales::text AS comentarios,
    r.haber_basico_diurno_snap::numeric AS haber_basico_diurno,
    r.haber_basico_nocturno_snap::numeric AS haber_basico_nocturno,
    r.bonos_snap AS bonos, r.tiempo_capacitacion_snap::int4 AS tiempo_capacitacion_dias,
    r.created_at::timestamptz AS creado,
    r.updated_at::timestamptz AS version,
    (SELECT rr.cantidad_original::int4 FROM requerimiento_referencia rr
      WHERE rr.requerimiento_id = r.id AND rr.origen = 'creacion'
      ORDER BY rr.created_at LIMIT 1) AS cantidad_original,
    (SELECT coalesce(sum(e.cantidad), 0)::int8 FROM requerimiento_entregas e
      JOIN requerimiento_entrega_decisiones d ON d.entrega_id = e.id AND d.estado = 'aceptada'
      WHERE e.requerimiento_id = r.id) AS total_aceptado,
    (SELECT coalesce(sum(e.cantidad), 0)::int8 FROM requerimiento_entregas e
      WHERE e.requerimiento_id = r.id) AS reclutados_informados,
    u.nombre::text AS solicitante_nombre, uc.nombre::text AS solicitante_cargo,
    u.division::text AS solicitante_division, u.region::text AS solicitante_region,
    sup.nombre::text AS inmediato_superior,
    s.nombre::text AS servicio_nombre, s.codigo::text AS servicio_codigo,
    c.nombre::text AS cargo_nombre, c.tipo::text AS cargo_tipo,
    cc.codigo || ' · ' || cc.nombre AS centro_costo, cl.nombre::text AS cliente,
    CASE WHEN k.nombre IS NULL THEN k.codigo ELSE k.codigo || ' · ' || k.nombre END AS contrato,
    s.tipo_contrato::text AS tipo_contrato, s.modalidad::text AS modalidad,
    s.tipo_turno::text AS tipo_turno, s.dias_trabajo AS dias_trabajo,
    s.departamento_region::text AS departamento_region
FROM requerimientos r
LEFT JOIN servicios s              ON s.id = r.servicio_id
LEFT JOIN cargos c                 ON c.id = r.cargo_id
LEFT JOIN centros_costo cc         ON cc.id = s.centro_costo_id
LEFT JOIN clientes cl              ON cl.id = s.cliente_id
LEFT JOIN contratos_comerciales k  ON k.id = s.contrato_comercial_id
LEFT JOIN usuarios u               ON u.id = r.solicitante_id
LEFT JOIN cargos uc                ON uc.id = u.cargo_id
LEFT JOIN usuarios sup             ON sup.id = u.inmediato_superior_id
WHERE r.id = $1 AND (r.servicio_id IS NULL OR s.activo IS TRUE)
"#;

#[derive(Debug, Clone, serde::Serialize, FromRow)]
pub struct DetalleRequerimiento {
    pub id: Uuid,
    pub numero: i64,
    pub folio: Option<String>,
    pub tipo: Option<String>,
    pub estado: Option<String>,
    pub motivo: Option<String>,
    pub cantidad: Option<i32>,
    pub fecha_inicio: NaiveDate,
    pub fecha_fin_tentativa: Option<NaiveDate>,
    pub fecha_inicio_capacitacion: Option<NaiveDate>,
    pub fecha_inicio_operacion: Option<NaiveDate>,
    pub comentarios: Option<String>,
    pub haber_basico_diurno: Option<Decimal>,
    pub haber_basico_nocturno: Option<Decimal>,
    /// Tal como lo guarda la base: `{metrica, monto}` (configurados) y
    /// `{tipo: "extra", nombre, monto, motivo}` (extra).
    pub bonos: Option<serde_json::Value>,
    pub tiempo_capacitacion_dias: Option<i32>,
    pub creado: DateTime<Utc>,
    /// `updated_at` con microsegundos: devolverlo tal cual al editar (concurrencia).
    pub version: DateTime<Utc>,
    /// Con cuánto arrancó (`requerimiento_referencia`, origen `creacion`); nunca se edita.
    pub cantidad_original: Option<i32>,
    /// Suma de entregas con decisión `aceptada` (mismo cálculo que usa la base al validar).
    pub total_aceptado: i64,
    /// A confirmar: suma de todas las entregas registradas (entregas y rectificaciones,
    /// con o sin decisión).
    pub reclutados_informados: i64,
    pub solicitante_nombre: Option<String>,
    pub solicitante_cargo: Option<String>,
    pub solicitante_division: Option<String>,
    pub solicitante_region: Option<String>,
    pub inmediato_superior: Option<String>,
    pub servicio_nombre: Option<String>,
    pub servicio_codigo: Option<String>,
    pub cargo_nombre: Option<String>,
    pub cargo_tipo: Option<String>,
    pub centro_costo: Option<String>,
    pub cliente: Option<String>,
    pub contrato: Option<String>,
    pub tipo_contrato: Option<String>,
    pub modalidad: Option<String>,
    pub tipo_turno: Option<String>,
    pub dias_trabajo: Option<Vec<String>>,
    pub departamento_region: Option<String>,
}

/// `None` si no existe o RLS no lo deja ver.
pub async fn detalle(
    conn: &mut PgConnection,
    id: Uuid,
) -> Result<Option<DetalleRequerimiento>, DbError> {
    Ok(sqlx::query_as(QUERY_DETALLE)
        .bind(id)
        .fetch_optional(conn)
        .await?)
}

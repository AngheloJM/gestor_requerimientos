//! Matriz de servicio sobre el schema real:
//! `servicio_cargos` (Composición Salarial) ⋈ `servicios` ⋈ `cargos`, con centro de costo,
//! cliente y proyecto opcionales; métricas (`servicio_cargo_metricas` + `catalogo_metricas`)
//! y umbrales (`servicio_cargo_metrica_umbrales`).
//!
//! Orden obligatorio de negocio: Centro de Costo → Cliente → Servicio → Cargo.
//! Solo servicios con `activo = true`. La visibilidad de filas la decide RLS.

use std::collections::HashMap;

use domain::{
    Cargo, CentroCosto, Cliente, ComposicionSalarial, Decimal, FilaMatriz, FiltrosMatriz,
    MetricaBono, Proyecto, Servicio, Umbral,
};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::DbError;

const QUERY_MATRIZ: &str = r#"
SELECT
    sc.id                              AS servicio_cargo_id,
    sc.haber_basico_diurno::numeric    AS haber_basico_diurno,
    sc.haber_basico_nocturno::numeric  AS haber_basico_nocturno,
    sc.cantidad_mujeres::int4          AS cantidad_mujeres,
    sc.cantidad_hombres::int4          AS cantidad_hombres,
    s.id               AS servicio_id,
    s.codigo::text     AS servicio_codigo,
    s.nombre::text     AS servicio_nombre,
    s.centro_costo_id,
    s.contrato_comercial_id,
    s.cliente_id,
    s.proyecto_id,
    s.activo           AS servicio_activo,
    c.id               AS cargo_id,
    c.nombre::text     AS cargo_nombre,
    c.tipo::text       AS cargo_tipo,
    cc.id              AS cc_id,
    cc.codigo::text    AS cc_codigo,
    cc.nombre::text    AS cc_nombre,
    cl.id              AS cl_id,
    cl.nombre::text    AS cl_nombre,
    cl.logo_url::text  AS cl_logo_url,
    cl.manual_marca_url::text AS cl_manual_marca_url,
    p.id               AS p_id,
    p.cliente_id       AS p_cliente_id,
    p.nombre::text     AS p_nombre
FROM servicio_cargos sc
JOIN servicios          s  ON s.id  = sc.servicio_id
JOIN cargos             c  ON c.id  = sc.cargo_id
LEFT JOIN centros_costo cc ON cc.id = s.centro_costo_id
LEFT JOIN clientes      cl ON cl.id = s.cliente_id
LEFT JOIN proyectos     p  ON p.id  = s.proyecto_id
WHERE s.activo = true
  AND ($1::uuid IS NULL OR s.id              = $1)
  AND ($2::uuid IS NULL OR s.cliente_id      = $2)
  AND ($3::uuid IS NULL OR s.centro_costo_id = $3)
  AND ($4::uuid IS NULL OR s.proyecto_id     = $4)
ORDER BY cc.codigo NULLS LAST, cl.nombre NULLS LAST, s.nombre, c.nombre,
         cc.id, cl.id, s.id, c.id, sc.id
"#;

const QUERY_METRICAS: &str = r#"
SELECT m.id, m.servicio_cargo_id, m.metrica_id,
       cm.nombre::text AS nombre, cm.concepto::text AS concepto, cm.formula::text AS formula,
       m.peso_pct::numeric AS peso_pct, m.monto_bono::numeric AS monto_bono
FROM servicio_cargo_metricas m
JOIN catalogo_metricas cm ON cm.id = m.metrica_id
WHERE m.servicio_cargo_id = ANY($1)
ORDER BY cm.nombre, m.id
"#;

const QUERY_UMBRALES: &str = r#"
SELECT u.servicio_cargo_metrica_id,
       u.intervalo_min::numeric AS intervalo_min, u.intervalo_max::numeric AS intervalo_max,
       u.pct_bono::numeric AS pct_bono, u.observaciones::text AS observaciones,
       u.orden::int4 AS orden
FROM servicio_cargo_metrica_umbrales u
WHERE u.servicio_cargo_metrica_id = ANY($1)
ORDER BY u.orden, u.id
"#;

#[derive(FromRow)]
struct FilaRow {
    servicio_cargo_id: Uuid,
    haber_basico_diurno: Decimal,
    haber_basico_nocturno: Option<Decimal>,
    cantidad_mujeres: Option<i32>,
    cantidad_hombres: Option<i32>,
    servicio_id: Uuid,
    servicio_codigo: Option<String>,
    servicio_nombre: String,
    centro_costo_id: Option<Uuid>,
    contrato_comercial_id: Option<Uuid>,
    cliente_id: Option<Uuid>,
    proyecto_id: Option<Uuid>,
    servicio_activo: bool,
    cargo_id: Uuid,
    cargo_nombre: String,
    cargo_tipo: Option<String>,
    cc_id: Option<Uuid>,
    cc_codigo: Option<String>,
    cc_nombre: Option<String>,
    cl_id: Option<Uuid>,
    cl_nombre: Option<String>,
    cl_logo_url: Option<String>,
    cl_manual_marca_url: Option<String>,
    p_id: Option<Uuid>,
    p_cliente_id: Option<Uuid>,
    p_nombre: Option<String>,
}

#[derive(FromRow)]
struct MetricaRow {
    id: Uuid,
    servicio_cargo_id: Uuid,
    metrica_id: Uuid,
    nombre: String,
    concepto: Option<String>,
    formula: Option<String>,
    peso_pct: Decimal,
    monto_bono: Decimal,
}

#[derive(FromRow)]
struct UmbralRow {
    servicio_cargo_metrica_id: Uuid,
    intervalo_min: Option<Decimal>,
    intervalo_max: Option<Decimal>,
    pct_bono: Decimal,
    observaciones: Option<String>,
    orden: i32,
}

/// Tres consultas (filas, métricas, umbrales); sin N+1.
pub async fn listar_matriz(
    conn: &mut PgConnection,
    f: FiltrosMatriz,
) -> Result<Vec<FilaMatriz>, DbError> {
    let filas: Vec<FilaRow> = sqlx::query_as(QUERY_MATRIZ)
        .bind(f.servicio_id.map(Uuid::from))
        .bind(f.cliente_id.map(Uuid::from))
        .bind(f.centro_costo_id.map(Uuid::from))
        .bind(f.proyecto_id.map(Uuid::from))
        .fetch_all(&mut *conn)
        .await?;
    if filas.is_empty() {
        return Ok(Vec::new());
    }

    let sc_ids: Vec<Uuid> = filas.iter().map(|r| r.servicio_cargo_id).collect();
    let metricas: Vec<MetricaRow> = sqlx::query_as(QUERY_METRICAS)
        .bind(&sc_ids)
        .fetch_all(&mut *conn)
        .await?;

    let metrica_ids: Vec<Uuid> = metricas.iter().map(|m| m.id).collect();
    let umbrales: Vec<UmbralRow> = if metrica_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as(QUERY_UMBRALES)
            .bind(&metrica_ids)
            .fetch_all(&mut *conn)
            .await?
    };

    let mut umbrales_por_metrica: HashMap<Uuid, Vec<Umbral>> = HashMap::new();
    for u in umbrales {
        umbrales_por_metrica
            .entry(u.servicio_cargo_metrica_id)
            .or_default()
            .push(Umbral {
                intervalo_min: u.intervalo_min,
                intervalo_max: u.intervalo_max,
                pct_bono: u.pct_bono,
                observaciones: u.observaciones,
                orden: u.orden,
            });
    }

    let mut metricas_por_sc: HashMap<Uuid, Vec<MetricaBono>> = HashMap::new();
    for m in metricas {
        metricas_por_sc
            .entry(m.servicio_cargo_id)
            .or_default()
            .push(MetricaBono {
                id: m.id.into(),
                metrica_id: m.metrica_id.into(),
                nombre: m.nombre,
                concepto: m.concepto,
                formula: m.formula,
                peso_pct: m.peso_pct,
                monto_bono: m.monto_bono,
                umbrales: umbrales_por_metrica.remove(&m.id).unwrap_or_default(),
            });
    }

    Ok(filas
        .into_iter()
        .map(|r| FilaMatriz {
            centro_costo: r.cc_id.map(|id| CentroCosto {
                id: id.into(),
                codigo: r.cc_codigo.unwrap_or_default(),
                nombre: r.cc_nombre.unwrap_or_default(),
            }),
            cliente: r.cl_id.map(|id| Cliente {
                id: id.into(),
                nombre: r.cl_nombre.unwrap_or_default(),
                logo_url: r.cl_logo_url,
                manual_marca_url: r.cl_manual_marca_url,
            }),
            proyecto: r.p_id.zip(r.p_cliente_id).map(|(id, cliente_id)| Proyecto {
                id: id.into(),
                cliente_id: cliente_id.into(),
                nombre: r.p_nombre.unwrap_or_default(),
            }),
            servicio: Servicio {
                id: r.servicio_id.into(),
                codigo: r.servicio_codigo,
                nombre: r.servicio_nombre,
                centro_costo_id: r.centro_costo_id.map(Into::into),
                contrato_comercial_id: r.contrato_comercial_id.map(Into::into),
                cliente_id: r.cliente_id.map(Into::into),
                proyecto_id: r.proyecto_id.map(Into::into),
                activo: r.servicio_activo,
            },
            cargo: Cargo {
                id: r.cargo_id.into(),
                nombre: r.cargo_nombre,
                tipo: r.cargo_tipo,
            },
            composicion: ComposicionSalarial {
                id: r.servicio_cargo_id.into(),
                servicio_id: r.servicio_id.into(),
                cargo_id: r.cargo_id.into(),
                haber_basico_diurno: r.haber_basico_diurno,
                haber_basico_nocturno: r.haber_basico_nocturno,
                cantidad_mujeres: r.cantidad_mujeres,
                cantidad_hombres: r.cantidad_hombres,
                metricas: metricas_por_sc
                    .remove(&r.servicio_cargo_id)
                    .unwrap_or_default(),
            },
        })
        .collect())
}

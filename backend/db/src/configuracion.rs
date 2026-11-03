//! Configuración de la Matriz de Servicio (secciones con alta/edición/baja).
//!
//! Autorización y reglas: la base. Lectura según RLS; escritura solo `rrhh_admin` (políticas
//! `*_write`). Validaciones en triggers de producción: vigencia de contratos, proyecto del
//! cliente del servicio, cliente de un proyecto con servicios, nombre de bono.
//!
//! Sin política de DELETE en producción (la base lo impide): `proyectos`, `catalogo_metricas`,
//! `catalogo_bonos`; `catalogo_metricas` tampoco tiene UPDATE. Por eso no hay funciones para
//! esas operaciones. TODO(negocio): agregar las políticas si se quieren habilitar.
//!
//! Las funciones de escritura devuelven filas afectadas: 0 = no existe o RLS no lo permite.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::DbError;

type R<T> = Result<T, DbError>;

async fn filas(
    q: sqlx::query::Query<'_, sqlx::Postgres, sqlx::postgres::PgArguments>,
    conn: &mut PgConnection,
) -> R<u64> {
    Ok(q.execute(conn).await?.rows_affected())
}

// --- 1. Clientes y proyectos ---------------------------------------------------------------

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Cliente {
    pub id: Uuid,
    pub nombre: String,
    pub logo_url: Option<String>,
    pub manual_marca_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatosCliente {
    pub nombre: String,
    /// TODO(negocio): hoy es texto/URL; subir el archivo en sí no está implementado.
    pub logo_url: Option<String>,
    pub manual_marca_url: Option<String>,
}

pub async fn listar_clientes(conn: &mut PgConnection) -> R<Vec<Cliente>> {
    Ok(sqlx::query_as(
        "SELECT id, nombre, logo_url, manual_marca_url FROM clientes ORDER BY lower(nombre)",
    )
    .fetch_all(conn)
    .await?)
}

pub async fn crear_cliente(conn: &mut PgConnection, d: &DatosCliente) -> R<Uuid> {
    Ok(sqlx::query_scalar("INSERT INTO clientes (nombre, logo_url, manual_marca_url) VALUES ($1, $2, $3) RETURNING id")
        .bind(&d.nombre)
        .bind(&d.logo_url)
        .bind(&d.manual_marca_url)
        .fetch_one(conn)
        .await?)
}

pub async fn actualizar_cliente(conn: &mut PgConnection, id: Uuid, d: &DatosCliente) -> R<u64> {
    filas(
        sqlx::query(
            "UPDATE clientes SET nombre = $2, logo_url = $3, manual_marca_url = $4 WHERE id = $1",
        )
        .bind(id)
        .bind(&d.nombre)
        .bind(&d.logo_url)
        .bind(&d.manual_marca_url),
        conn,
    )
    .await
}

pub async fn eliminar_cliente(conn: &mut PgConnection, id: Uuid) -> R<u64> {
    filas(
        sqlx::query("DELETE FROM clientes WHERE id = $1").bind(id),
        conn,
    )
    .await
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Proyecto {
    pub id: Uuid,
    pub cliente_id: Uuid,
    pub cliente_nombre: Option<String>,
    pub nombre: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatosProyecto {
    pub cliente_id: Uuid,
    pub nombre: String,
}

pub async fn listar_proyectos(conn: &mut PgConnection) -> R<Vec<Proyecto>> {
    Ok(sqlx::query_as(
        "SELECT p.id, p.cliente_id, c.nombre AS cliente_nombre, p.nombre
         FROM proyectos p LEFT JOIN clientes c ON c.id = p.cliente_id
         ORDER BY lower(c.nombre), lower(p.nombre)",
    )
    .fetch_all(conn)
    .await?)
}

pub async fn crear_proyecto(conn: &mut PgConnection, d: &DatosProyecto) -> R<Uuid> {
    Ok(sqlx::query_scalar(
        "INSERT INTO proyectos (cliente_id, nombre) VALUES ($1, $2) RETURNING id",
    )
    .bind(d.cliente_id)
    .bind(&d.nombre)
    .fetch_one(conn)
    .await?)
}

pub async fn actualizar_proyecto(conn: &mut PgConnection, id: Uuid, d: &DatosProyecto) -> R<u64> {
    filas(
        sqlx::query("UPDATE proyectos SET cliente_id = $2, nombre = $3 WHERE id = $1")
            .bind(id)
            .bind(d.cliente_id)
            .bind(&d.nombre),
        conn,
    )
    .await
}

// --- 2. Centros de costo -----------------------------------------------------------------

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct CentroCosto {
    pub id: Uuid,
    pub codigo: String,
    pub nombre: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatosCentroCosto {
    pub codigo: String,
    pub nombre: String,
}

pub async fn listar_centros(conn: &mut PgConnection) -> R<Vec<CentroCosto>> {
    Ok(
        sqlx::query_as("SELECT id, codigo, nombre FROM centros_costo ORDER BY codigo")
            .fetch_all(conn)
            .await?,
    )
}

pub async fn crear_centro(conn: &mut PgConnection, d: &DatosCentroCosto) -> R<Uuid> {
    Ok(sqlx::query_scalar(
        "INSERT INTO centros_costo (codigo, nombre) VALUES ($1, $2) RETURNING id",
    )
    .bind(&d.codigo)
    .bind(&d.nombre)
    .fetch_one(conn)
    .await?)
}

pub async fn actualizar_centro(conn: &mut PgConnection, id: Uuid, d: &DatosCentroCosto) -> R<u64> {
    filas(
        sqlx::query("UPDATE centros_costo SET codigo = $2, nombre = $3 WHERE id = $1")
            .bind(id)
            .bind(&d.codigo)
            .bind(&d.nombre),
        conn,
    )
    .await
}

pub async fn eliminar_centro(conn: &mut PgConnection, id: Uuid) -> R<u64> {
    filas(
        sqlx::query("DELETE FROM centros_costo WHERE id = $1").bind(id),
        conn,
    )
    .await
}

// --- 3. Servicios ----------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ServicioConfig {
    pub id: Uuid,
    pub codigo: Option<String>,
    pub nombre: String,
    pub centro_costo_id: Option<Uuid>,
    pub centro_costo_nombre: Option<String>,
    pub contrato_comercial_id: Option<Uuid>,
    pub contrato_nombre: Option<String>,
    pub cliente_id: Option<Uuid>,
    pub cliente_nombre: Option<String>,
    pub proyecto_id: Option<Uuid>,
    pub proyecto_nombre: Option<String>,
    pub tipo_contrato: Option<String>,
    pub modalidad: String,
    pub tipo_turno: Option<String>,
    pub dias_trabajo: Vec<String>,
    pub departamento_region: Option<String>,
    pub manual_funciones_codigo: Option<String>,
    pub tiempo_capacitacion_dias: Option<i32>,
    pub capacitacion_pagada: Option<bool>,
    pub tiene_refrigerio: Option<bool>,
    pub activo: bool,
}

/// `cliente_id` y `centro_costo_id` son independientes (uno no implica al otro).
#[derive(Debug, Clone, Deserialize)]
pub struct DatosServicio {
    pub codigo: Option<String>,
    pub nombre: String,
    pub centro_costo_id: Option<Uuid>,
    pub contrato_comercial_id: Option<Uuid>,
    pub cliente_id: Option<Uuid>,
    /// Debe pertenecer al cliente elegido (lo valida un trigger de la base).
    pub proyecto_id: Option<Uuid>,
    pub tipo_contrato: Option<String>,
    pub modalidad: String,
    pub tipo_turno: Option<String>,
    pub dias_trabajo: Vec<String>,
    pub departamento_region: Option<String>,
    pub manual_funciones_codigo: Option<String>,
    pub tiempo_capacitacion_dias: Option<i32>,
    pub capacitacion_pagada: bool,
    pub tiene_refrigerio: bool,
    pub activo: bool,
}

pub async fn listar_servicios(conn: &mut PgConnection) -> R<Vec<ServicioConfig>> {
    Ok(sqlx::query_as(
        "SELECT s.id, s.codigo, s.nombre, s.centro_costo_id,
                cc.codigo || ' · ' || cc.nombre AS centro_costo_nombre,
                s.contrato_comercial_id, coalesce(k.nombre, k.codigo) AS contrato_nombre,
                s.cliente_id, cl.nombre AS cliente_nombre, s.proyecto_id, p.nombre AS proyecto_nombre,
                s.tipo_contrato, s.modalidad, s.tipo_turno, s.dias_trabajo, s.departamento_region,
                s.manual_funciones_codigo, s.tiempo_capacitacion_dias, s.capacitacion_pagada,
                s.tiene_refrigerio, s.activo
         FROM servicios s
         LEFT JOIN centros_costo cc ON cc.id = s.centro_costo_id
         LEFT JOIN contratos_comerciales k ON k.id = s.contrato_comercial_id
         LEFT JOIN clientes cl ON cl.id = s.cliente_id
         LEFT JOIN proyectos p ON p.id = s.proyecto_id
         ORDER BY s.activo DESC, lower(s.nombre)",
    )
    .fetch_all(conn)
    .await?)
}

fn bind_servicio<'q>(
    q: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
    d: &'q DatosServicio,
) -> sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments> {
    q.bind(&d.codigo)
        .bind(&d.nombre)
        .bind(d.centro_costo_id)
        .bind(d.contrato_comercial_id)
        .bind(d.cliente_id)
        .bind(d.proyecto_id)
        .bind(&d.tipo_contrato)
        .bind(&d.modalidad)
        .bind(&d.tipo_turno)
        .bind(&d.dias_trabajo)
        .bind(&d.departamento_region)
        .bind(&d.manual_funciones_codigo)
        .bind(d.tiempo_capacitacion_dias)
        .bind(d.capacitacion_pagada)
        .bind(d.tiene_refrigerio)
        .bind(d.activo)
}

pub async fn crear_servicio(conn: &mut PgConnection, d: &DatosServicio) -> R<Uuid> {
    let fila: (Uuid,) = sqlx::query_as(
        "INSERT INTO servicios (codigo, nombre, centro_costo_id, contrato_comercial_id, cliente_id, proyecto_id,
            tipo_contrato, modalidad, tipo_turno, dias_trabajo, departamento_region, manual_funciones_codigo,
            tiempo_capacitacion_dias, capacitacion_pagada, tiene_refrigerio, activo)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16) RETURNING id",
    )
    .bind(&d.codigo)
    .bind(&d.nombre)
    .bind(d.centro_costo_id)
    .bind(d.contrato_comercial_id)
    .bind(d.cliente_id)
    .bind(d.proyecto_id)
    .bind(&d.tipo_contrato)
    .bind(&d.modalidad)
    .bind(&d.tipo_turno)
    .bind(&d.dias_trabajo)
    .bind(&d.departamento_region)
    .bind(&d.manual_funciones_codigo)
    .bind(d.tiempo_capacitacion_dias)
    .bind(d.capacitacion_pagada)
    .bind(d.tiene_refrigerio)
    .bind(d.activo)
    .fetch_one(conn)
    .await?;
    Ok(fila.0)
}

pub async fn actualizar_servicio(conn: &mut PgConnection, id: Uuid, d: &DatosServicio) -> R<u64> {
    let q = sqlx::query(
        "UPDATE servicios SET codigo = $1, nombre = $2, centro_costo_id = $3, contrato_comercial_id = $4,
            cliente_id = $5, proyecto_id = $6, tipo_contrato = $7, modalidad = $8, tipo_turno = $9,
            dias_trabajo = $10, departamento_region = $11, manual_funciones_codigo = $12,
            tiempo_capacitacion_dias = $13, capacitacion_pagada = $14, tiene_refrigerio = $15, activo = $16
         WHERE id = $17",
    );
    filas(bind_servicio(q, d).bind(id), conn).await
}

/// Borrado físico: la base lo rechaza (FK) si el servicio tiene cargos, requerimientos, etc.
/// Para dejar de usarlo, `activo = false`.
pub async fn eliminar_servicio(conn: &mut PgConnection, id: Uuid) -> R<u64> {
    filas(
        sqlx::query("DELETE FROM servicios WHERE id = $1").bind(id),
        conn,
    )
    .await
}

/// Catálogos de producción usados por el formulario de servicios (solo activos).
pub async fn catalogos_servicio(conn: &mut PgConnection) -> R<(Vec<String>, Vec<String>)> {
    let tipos: Vec<String> =
        sqlx::query_scalar("SELECT nombre FROM tipos_contrato WHERE activo ORDER BY nombre")
            .fetch_all(&mut *conn)
            .await?;
    let modalidades: Vec<String> =
        sqlx::query_scalar("SELECT nombre FROM modalidades WHERE activo ORDER BY nombre")
            .fetch_all(conn)
            .await?;
    Ok((tipos, modalidades))
}

// --- 4. Cargos -------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct CargoConfig {
    pub id: Uuid,
    pub nombre: String,
    pub tipo: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatosCargo {
    pub nombre: String,
    /// `operativo` | `administrativo_staff` (CHECK de la base).
    pub tipo: String,
}

pub async fn listar_cargos(conn: &mut PgConnection) -> R<Vec<CargoConfig>> {
    Ok(
        sqlx::query_as("SELECT id, nombre, tipo FROM cargos ORDER BY lower(nombre), tipo")
            .fetch_all(conn)
            .await?,
    )
}

pub async fn crear_cargo(conn: &mut PgConnection, d: &DatosCargo) -> R<Uuid> {
    Ok(
        sqlx::query_scalar("INSERT INTO cargos (nombre, tipo) VALUES ($1, $2) RETURNING id")
            .bind(&d.nombre)
            .bind(&d.tipo)
            .fetch_one(conn)
            .await?,
    )
}

pub async fn actualizar_cargo(conn: &mut PgConnection, id: Uuid, d: &DatosCargo) -> R<u64> {
    filas(
        sqlx::query("UPDATE cargos SET nombre = $2, tipo = $3 WHERE id = $1")
            .bind(id)
            .bind(&d.nombre)
            .bind(&d.tipo),
        conn,
    )
    .await
}

pub async fn eliminar_cargo(conn: &mut PgConnection, id: Uuid) -> R<u64> {
    filas(
        sqlx::query("DELETE FROM cargos WHERE id = $1").bind(id),
        conn,
    )
    .await
}

// --- 5. Composición salarial (servicio_cargos) -------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct DatosComposicion {
    pub haber_basico_diurno: Decimal,
    pub haber_basico_nocturno: Option<Decimal>,
}

pub async fn crear_composicion(
    conn: &mut PgConnection,
    servicio_id: Uuid,
    cargo_id: Uuid,
    d: &DatosComposicion,
) -> R<Uuid> {
    Ok(sqlx::query_scalar(
        "INSERT INTO servicio_cargos (servicio_id, cargo_id, haber_basico_diurno, haber_basico_nocturno)
         VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(servicio_id)
    .bind(cargo_id)
    .bind(d.haber_basico_diurno)
    .bind(d.haber_basico_nocturno)
    .fetch_one(conn)
    .await?)
}

pub async fn actualizar_composicion(
    conn: &mut PgConnection,
    id: Uuid,
    d: &DatosComposicion,
) -> R<u64> {
    filas(
        sqlx::query("UPDATE servicio_cargos SET haber_basico_diurno = $2, haber_basico_nocturno = $3 WHERE id = $1")
            .bind(id)
            .bind(d.haber_basico_diurno)
            .bind(d.haber_basico_nocturno),
        conn,
    )
    .await
}

// --- 6. Contratos ----------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Contrato {
    pub id: Uuid,
    pub codigo: String,
    pub nombre: Option<String>,
    pub cliente_id: Option<Uuid>,
    pub cliente_nombre: Option<String>,
    pub fecha_inicio: Option<NaiveDate>,
    pub fecha_fin: Option<NaiveDate>,
    pub duracion_valor: Option<i32>,
    pub duracion_unidad: Option<String>,
}

/// `fecha_fin` y (`duracion_valor` + `duracion_unidad`) son alternativas (trigger de la base:
/// unidad `meses` | `dias`, inicio obligatorio, fin no anterior al inicio).
#[derive(Debug, Clone, Deserialize)]
pub struct DatosContrato {
    pub codigo: String,
    pub nombre: Option<String>,
    pub cliente_id: Option<Uuid>,
    pub fecha_inicio: Option<NaiveDate>,
    pub fecha_fin: Option<NaiveDate>,
    pub duracion_valor: Option<i32>,
    pub duracion_unidad: Option<String>,
}

pub async fn listar_contratos(conn: &mut PgConnection) -> R<Vec<Contrato>> {
    Ok(sqlx::query_as(
        "SELECT k.id, k.codigo, k.nombre, k.cliente_id, c.nombre AS cliente_nombre, k.fecha_inicio,
                k.fecha_fin, k.duracion_valor, k.duracion_unidad
         FROM contratos_comerciales k LEFT JOIN clientes c ON c.id = k.cliente_id
         ORDER BY k.codigo",
    )
    .fetch_all(conn)
    .await?)
}

pub async fn crear_contrato(conn: &mut PgConnection, d: &DatosContrato) -> R<Uuid> {
    Ok(sqlx::query_scalar(
        "INSERT INTO contratos_comerciales (codigo, nombre, cliente_id, fecha_inicio, fecha_fin, duracion_valor, duracion_unidad)
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
    )
    .bind(&d.codigo)
    .bind(&d.nombre)
    .bind(d.cliente_id)
    .bind(d.fecha_inicio)
    .bind(d.fecha_fin)
    .bind(d.duracion_valor)
    .bind(&d.duracion_unidad)
    .fetch_one(conn)
    .await?)
}

pub async fn actualizar_contrato(conn: &mut PgConnection, id: Uuid, d: &DatosContrato) -> R<u64> {
    filas(
        sqlx::query(
            "UPDATE contratos_comerciales SET codigo = $2, nombre = $3, cliente_id = $4, fecha_inicio = $5,
                fecha_fin = $6, duracion_valor = $7, duracion_unidad = $8 WHERE id = $1",
        )
        .bind(id)
        .bind(&d.codigo)
        .bind(&d.nombre)
        .bind(d.cliente_id)
        .bind(d.fecha_inicio)
        .bind(d.fecha_fin)
        .bind(d.duracion_valor)
        .bind(&d.duracion_unidad),
        conn,
    )
    .await
}

pub async fn eliminar_contrato(conn: &mut PgConnection, id: Uuid) -> R<u64> {
    filas(
        sqlx::query("DELETE FROM contratos_comerciales WHERE id = $1").bind(id),
        conn,
    )
    .await
}

// --- 7. Métricas y bonos por cargo (servicio_cargo_metricas + umbrales) -------------------

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UmbralConfig {
    pub intervalo_min: Option<Decimal>,
    pub intervalo_max: Option<Decimal>,
    /// TODO(negocio): escala de `pct_bono` (0–100 o 0–1) sin confirmar; se guarda tal cual.
    pub pct_bono: Decimal,
    pub observaciones: Option<String>,
    pub orden: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricaAsignada {
    pub id: Uuid,
    pub metrica_id: Uuid,
    pub nombre: String,
    pub peso_pct: Decimal,
    pub monto_bono: Decimal,
    pub umbrales: Vec<UmbralConfig>,
}

#[derive(FromRow)]
struct MetricaRow {
    id: Uuid,
    metrica_id: Uuid,
    nombre: String,
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

pub async fn metricas_de(
    conn: &mut PgConnection,
    servicio_cargo_id: Uuid,
) -> R<Vec<MetricaAsignada>> {
    let metricas: Vec<MetricaRow> = sqlx::query_as(
        "SELECT m.id, m.metrica_id, cm.nombre, m.peso_pct, m.monto_bono
         FROM servicio_cargo_metricas m JOIN catalogo_metricas cm ON cm.id = m.metrica_id
         WHERE m.servicio_cargo_id = $1 ORDER BY lower(cm.nombre)",
    )
    .bind(servicio_cargo_id)
    .fetch_all(&mut *conn)
    .await?;
    let ids: Vec<Uuid> = metricas.iter().map(|m| m.id).collect();
    let umbrales: Vec<UmbralRow> = sqlx::query_as(
        "SELECT servicio_cargo_metrica_id, intervalo_min, intervalo_max, pct_bono, observaciones, orden
         FROM servicio_cargo_metrica_umbrales WHERE servicio_cargo_metrica_id = ANY($1)
         ORDER BY orden, intervalo_min NULLS FIRST",
    )
    .bind(&ids)
    .fetch_all(conn)
    .await?;
    Ok(metricas
        .into_iter()
        .map(|m| MetricaAsignada {
            umbrales: umbrales
                .iter()
                .filter(|u| u.servicio_cargo_metrica_id == m.id)
                .map(|u| UmbralConfig {
                    intervalo_min: u.intervalo_min,
                    intervalo_max: u.intervalo_max,
                    pct_bono: u.pct_bono,
                    observaciones: u.observaciones.clone(),
                    orden: u.orden,
                })
                .collect(),
            id: m.id,
            metrica_id: m.metrica_id,
            nombre: m.nombre,
            peso_pct: m.peso_pct,
            monto_bono: m.monto_bono,
        })
        .collect())
}

pub async fn asignar_metrica(
    conn: &mut PgConnection,
    servicio_cargo_id: Uuid,
    metrica_id: Uuid,
    peso_pct: Decimal,
    monto_bono: Decimal,
) -> R<Uuid> {
    Ok(sqlx::query_scalar(
        "INSERT INTO servicio_cargo_metricas (servicio_cargo_id, metrica_id, peso_pct, monto_bono)
         VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(servicio_cargo_id)
    .bind(metrica_id)
    .bind(peso_pct)
    .bind(monto_bono)
    .fetch_one(conn)
    .await?)
}

/// Actualiza peso y monto y REEMPLAZA la tabla de escalas (borra y vuelve a insertar).
pub async fn actualizar_metrica(
    conn: &mut PgConnection,
    id: Uuid,
    peso_pct: Decimal,
    monto_bono: Decimal,
    umbrales: &[UmbralConfig],
) -> R<u64> {
    let n = sqlx::query(
        "UPDATE servicio_cargo_metricas SET peso_pct = $2, monto_bono = $3 WHERE id = $1",
    )
    .bind(id)
    .bind(peso_pct)
    .bind(monto_bono)
    .execute(&mut *conn)
    .await?
    .rows_affected();
    if n == 0 {
        return Ok(0);
    }
    sqlx::query("DELETE FROM servicio_cargo_metrica_umbrales WHERE servicio_cargo_metrica_id = $1")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    for u in umbrales {
        sqlx::query(
            "INSERT INTO servicio_cargo_metrica_umbrales
                (servicio_cargo_metrica_id, intervalo_min, intervalo_max, pct_bono, observaciones, orden)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(id)
        .bind(u.intervalo_min)
        .bind(u.intervalo_max)
        .bind(u.pct_bono)
        .bind(&u.observaciones)
        .bind(u.orden)
        .execute(&mut *conn)
        .await?;
    }
    Ok(n)
}

/// Quita la métrica del servicio_cargo (sus escalas se borran en cascada: FK de la base).
pub async fn quitar_metrica(conn: &mut PgConnection, id: Uuid) -> R<u64> {
    filas(
        sqlx::query("DELETE FROM servicio_cargo_metricas WHERE id = $1").bind(id),
        conn,
    )
    .await
}

// --- 8. Catálogo de métricas (solo alta: no hay UPDATE/DELETE en producción) --------------

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct MetricaCatalogo {
    pub id: Uuid,
    pub nombre: String,
    pub concepto: Option<String>,
    pub formula: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatosMetricaCatalogo {
    pub nombre: String,
    pub concepto: Option<String>,
    pub formula: Option<String>,
}

pub async fn listar_metricas_catalogo(conn: &mut PgConnection) -> R<Vec<MetricaCatalogo>> {
    Ok(sqlx::query_as(
        "SELECT id, nombre, concepto, formula FROM catalogo_metricas ORDER BY lower(nombre)",
    )
    .fetch_all(conn)
    .await?)
}

pub async fn crear_metrica_catalogo(conn: &mut PgConnection, d: &DatosMetricaCatalogo) -> R<Uuid> {
    Ok(sqlx::query_scalar("INSERT INTO catalogo_metricas (nombre, concepto, formula) VALUES ($1, $2, $3) RETURNING id")
        .bind(&d.nombre)
        .bind(&d.concepto)
        .bind(&d.formula)
        .fetch_one(conn)
        .await?)
}

// --- 9. Catálogo de bonos (alta y edición; no hay DELETE en producción) -------------------
// TODO(negocio): no hay tabla puente que lo conecte con nada (las métricas usan metrica_id).

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct BonoCatalogo {
    pub id: Uuid,
    pub nombre: String,
}

pub async fn listar_bonos(conn: &mut PgConnection) -> R<Vec<BonoCatalogo>> {
    Ok(
        sqlx::query_as("SELECT id, nombre FROM catalogo_bonos ORDER BY lower(nombre)")
            .fetch_all(conn)
            .await?,
    )
}

pub async fn crear_bono(conn: &mut PgConnection, nombre: &str) -> R<Uuid> {
    Ok(
        sqlx::query_scalar("INSERT INTO catalogo_bonos (nombre) VALUES ($1) RETURNING id")
            .bind(nombre)
            .fetch_one(conn)
            .await?,
    )
}

pub async fn actualizar_bono(conn: &mut PgConnection, id: Uuid, nombre: &str) -> R<u64> {
    filas(
        sqlx::query("UPDATE catalogo_bonos SET nombre = $2 WHERE id = $1")
            .bind(id)
            .bind(nombre),
        conn,
    )
    .await
}

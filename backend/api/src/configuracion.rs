//! Configuración de la Matriz de Servicio (alta/edición/baja de cada sección).
//! Escritura: solo RR.HH. (RLS). Reglas: triggers de la base. Aquí se validan formatos y la
//! exclusión fecha de fin / duración de contratos, para responder 422 claros.
//!
//! Sin endpoints de baja para proyectos, catálogo de métricas y catálogo de bonos, ni de
//! edición del catálogo de métricas: producción no tiene políticas RLS para eso.
//! TODO(negocio): habilitarlas requiere agregar políticas en la base.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use db::{DbError, PgPool, configuracion as cfg};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::{ApiError, ApiResult, tx_usuario},
};

#[derive(Debug, Serialize)]
pub struct Creado {
    pub id: Uuid,
}

/// Días admitidos en `servicios.dias_trabajo` (valores usados en producción).
pub const DIAS: [&str; 7] = [
    "Lunes",
    "Martes",
    "Miércoles",
    "Jueves",
    "Viernes",
    "Sábado",
    "Domingo",
];
/// CHECK `servicios_modalidad_check`.
const MODALIDADES: [&str; 6] = [
    "Presencial",
    "Remoto",
    "Híbrido",
    "presencial",
    "remoto",
    "híbrido",
];
/// CHECK `servicios_tipo_turno_check`.
const TIPOS_TURNO: [&str; 2] = ["continuo", "turno"];
/// CHECK `cargos_tipo_check`.
const TIPOS_CARGO: [&str; 2] = ["operativo", "administrativo_staff"];
/// Trigger `validar_vigencia_contrato_comercial`.
const UNIDADES_DURACION: [&str; 2] = ["meses", "dias"];

fn error(e: DbError) -> ApiError {
    if e.es_duplicado() {
        ApiError::Conflict("ya existe un registro con ese código o nombre".into())
    } else if e.es_fk() {
        ApiError::Conflict("está en uso por otros registros (o una referencia no existe)".into())
    } else if let Some(m) = e.check_violado() {
        ApiError::Unprocessable(m.to_owned())
    } else {
        e.into()
    }
}

fn una_fila(n: u64) -> ApiResult<StatusCode> {
    if n == 0 {
        Err(ApiError::NotFound)
    } else {
        Ok(StatusCode::NO_CONTENT)
    }
}

fn invalido(m: impl Into<String>) -> ApiError {
    ApiError::Unprocessable(m.into())
}

/// Texto obligatorio (recortado, 1..=max caracteres).
fn requerido(campo: &str, v: &str, max: usize) -> Result<String, ApiError> {
    let v = v.split_whitespace().collect::<Vec<_>>().join(" ");
    if v.is_empty() || v.chars().count() > max {
        return Err(invalido(format!(
            "{campo}: obligatorio, hasta {max} caracteres"
        )));
    }
    Ok(v)
}

/// Texto opcional: vacío → `None`.
fn opcional(campo: &str, v: &Option<String>, max: usize) -> Result<Option<String>, ApiError> {
    let v = v.as_deref().map(str::trim).filter(|s| !s.is_empty());
    if v.is_some_and(|s| s.chars().count() > max) {
        return Err(invalido(format!("{campo}: hasta {max} caracteres")));
    }
    Ok(v.map(str::to_owned))
}

fn no_negativo(campo: &str, v: Decimal) -> Result<Decimal, ApiError> {
    if v.is_sign_negative() {
        return Err(invalido(format!("{campo}: no puede ser negativo")));
    }
    Ok(v)
}

macro_rules! tx {
    ($pool:expr, $user:expr) => {
        tx_usuario(&$pool, &$user).await?
    };
}

// --- Clientes y proyectos ------------------------------------------------------------------

fn validar_cliente(d: cfg::DatosCliente) -> Result<cfg::DatosCliente, ApiError> {
    Ok(cfg::DatosCliente {
        nombre: requerido("nombre", &d.nombre, 200)?,
        logo_url: opcional("logo", &d.logo_url, 2000)?,
        manual_marca_url: opcional("manual de marca", &d.manual_marca_url, 2000)?,
    })
}

pub async fn listar_clientes(
    State(pool): State<PgPool>,
    user: AuthUser,
) -> ApiResult<Json<Vec<cfg::Cliente>>> {
    let mut tx = tx!(pool, user);
    Ok(Json(cfg::listar_clientes(&mut tx).await?))
}

pub async fn crear_cliente(
    State(pool): State<PgPool>,
    user: AuthUser,
    Json(d): Json<cfg::DatosCliente>,
) -> ApiResult<(StatusCode, Json<Creado>)> {
    let d = validar_cliente(d)?;
    let mut tx = tx!(pool, user);
    let id = cfg::crear_cliente(&mut tx, &d).await.map_err(error)?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(Creado { id })))
}

pub async fn actualizar_cliente(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(d): Json<cfg::DatosCliente>,
) -> ApiResult<StatusCode> {
    let d = validar_cliente(d)?;
    let mut tx = tx!(pool, user);
    let r = una_fila(
        cfg::actualizar_cliente(&mut tx, id, &d)
            .await
            .map_err(error)?,
    )?;
    tx.commit().await?;
    Ok(r)
}

pub async fn eliminar_cliente(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let mut tx = tx!(pool, user);
    let r = una_fila(cfg::eliminar_cliente(&mut tx, id).await.map_err(error)?)?;
    tx.commit().await?;
    Ok(r)
}

fn validar_proyecto(d: cfg::DatosProyecto) -> Result<cfg::DatosProyecto, ApiError> {
    Ok(cfg::DatosProyecto {
        cliente_id: d.cliente_id,
        nombre: requerido("nombre", &d.nombre, 200)?,
    })
}

pub async fn listar_proyectos(
    State(pool): State<PgPool>,
    user: AuthUser,
) -> ApiResult<Json<Vec<cfg::Proyecto>>> {
    let mut tx = tx!(pool, user);
    Ok(Json(cfg::listar_proyectos(&mut tx).await?))
}

pub async fn crear_proyecto(
    State(pool): State<PgPool>,
    user: AuthUser,
    Json(d): Json<cfg::DatosProyecto>,
) -> ApiResult<(StatusCode, Json<Creado>)> {
    let d = validar_proyecto(d)?;
    let mut tx = tx!(pool, user);
    let id = cfg::crear_proyecto(&mut tx, &d).await.map_err(error)?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(Creado { id })))
}

pub async fn actualizar_proyecto(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(d): Json<cfg::DatosProyecto>,
) -> ApiResult<StatusCode> {
    let d = validar_proyecto(d)?;
    let mut tx = tx!(pool, user);
    let r = una_fila(
        cfg::actualizar_proyecto(&mut tx, id, &d)
            .await
            .map_err(error)?,
    )?;
    tx.commit().await?;
    Ok(r)
}

// --- Centros de costo ----------------------------------------------------------------------

fn validar_centro(d: cfg::DatosCentroCosto) -> Result<cfg::DatosCentroCosto, ApiError> {
    Ok(cfg::DatosCentroCosto {
        codigo: requerido("código", &d.codigo, 60)?,
        nombre: requerido("nombre", &d.nombre, 200)?,
    })
}

pub async fn listar_centros(
    State(pool): State<PgPool>,
    user: AuthUser,
) -> ApiResult<Json<Vec<cfg::CentroCosto>>> {
    let mut tx = tx!(pool, user);
    Ok(Json(cfg::listar_centros(&mut tx).await?))
}

pub async fn crear_centro(
    State(pool): State<PgPool>,
    user: AuthUser,
    Json(d): Json<cfg::DatosCentroCosto>,
) -> ApiResult<(StatusCode, Json<Creado>)> {
    let d = validar_centro(d)?;
    let mut tx = tx!(pool, user);
    let id = cfg::crear_centro(&mut tx, &d).await.map_err(error)?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(Creado { id })))
}

pub async fn actualizar_centro(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(d): Json<cfg::DatosCentroCosto>,
) -> ApiResult<StatusCode> {
    let d = validar_centro(d)?;
    let mut tx = tx!(pool, user);
    let r = una_fila(
        cfg::actualizar_centro(&mut tx, id, &d)
            .await
            .map_err(error)?,
    )?;
    tx.commit().await?;
    Ok(r)
}

pub async fn eliminar_centro(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let mut tx = tx!(pool, user);
    let r = una_fila(cfg::eliminar_centro(&mut tx, id).await.map_err(error)?)?;
    tx.commit().await?;
    Ok(r)
}

// --- Servicios -------------------------------------------------------------------------

fn validar_servicio(d: cfg::DatosServicio) -> Result<cfg::DatosServicio, ApiError> {
    if !MODALIDADES.contains(&d.modalidad.as_str()) {
        return Err(invalido(format!("modalidad inválida: {}", d.modalidad)));
    }
    if let Some(t) = d.tipo_turno.as_deref().filter(|t| !t.is_empty())
        && !TIPOS_TURNO.contains(&t)
    {
        return Err(invalido(format!("tipo de turno inválido: {t}")));
    }
    let mut dias = Vec::new();
    for dia in d.dias_trabajo {
        if !DIAS.contains(&dia.as_str()) {
            return Err(invalido(format!("día inválido: {dia}")));
        }
        if !dias.contains(&dia) {
            dias.push(dia);
        }
    }
    // Orden de la semana.
    dias.sort_by_key(|d| DIAS.iter().position(|x| x == d));
    if d.tiempo_capacitacion_dias
        .is_some_and(|v| !(0..=3650).contains(&v))
    {
        return Err(invalido("tiempo de capacitación: entre 0 y 3650 días"));
    }
    Ok(cfg::DatosServicio {
        codigo: opcional("código", &d.codigo, 60)?,
        nombre: requerido("nombre", &d.nombre, 200)?,
        tipo_contrato: opcional("tipo de contrato", &d.tipo_contrato, 120)?,
        tipo_turno: d.tipo_turno.filter(|t| !t.is_empty()),
        dias_trabajo: dias,
        departamento_region: opcional("departamento/región", &d.departamento_region, 200)?,
        manual_funciones_codigo: opcional(
            "código de manual de funciones",
            &d.manual_funciones_codigo,
            120,
        )?,
        ..d
    })
}

#[derive(Debug, Serialize)]
pub struct CatalogosServicio {
    pub tipos_contrato: Vec<String>,
    pub modalidades: Vec<String>,
    pub tipos_turno: [&'static str; 2],
    pub dias: [&'static str; 7],
}

pub async fn catalogos_servicio(
    State(pool): State<PgPool>,
    user: AuthUser,
) -> ApiResult<Json<CatalogosServicio>> {
    let mut tx = tx!(pool, user);
    let (tipos_contrato, modalidades) = cfg::catalogos_servicio(&mut tx).await?;
    Ok(Json(CatalogosServicio {
        tipos_contrato,
        modalidades,
        tipos_turno: TIPOS_TURNO,
        dias: DIAS,
    }))
}

pub async fn listar_servicios(
    State(pool): State<PgPool>,
    user: AuthUser,
) -> ApiResult<Json<Vec<cfg::ServicioConfig>>> {
    let mut tx = tx!(pool, user);
    Ok(Json(cfg::listar_servicios(&mut tx).await?))
}

pub async fn crear_servicio(
    State(pool): State<PgPool>,
    user: AuthUser,
    Json(d): Json<cfg::DatosServicio>,
) -> ApiResult<(StatusCode, Json<Creado>)> {
    let d = validar_servicio(d)?;
    let mut tx = tx!(pool, user);
    let id = cfg::crear_servicio(&mut tx, &d).await.map_err(error)?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(Creado { id })))
}

pub async fn actualizar_servicio(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(d): Json<cfg::DatosServicio>,
) -> ApiResult<StatusCode> {
    let d = validar_servicio(d)?;
    let mut tx = tx!(pool, user);
    let r = una_fila(
        cfg::actualizar_servicio(&mut tx, id, &d)
            .await
            .map_err(error)?,
    )?;
    tx.commit().await?;
    Ok(r)
}

pub async fn eliminar_servicio(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let mut tx = tx!(pool, user);
    let r = una_fila(cfg::eliminar_servicio(&mut tx, id).await.map_err(|e| {
        if e.es_fk() {
            ApiError::Conflict("el servicio está en uso (cargos, requerimientos…): desactivalo en lugar de eliminarlo".into())
        } else {
            error(e)
        }
    })?)?;
    tx.commit().await?;
    Ok(r)
}

// --- Cargos ----------------------------------------------------------------------------

fn validar_cargo(d: cfg::DatosCargo) -> Result<cfg::DatosCargo, ApiError> {
    if !TIPOS_CARGO.contains(&d.tipo.as_str()) {
        return Err(invalido(format!("tipo de cargo inválido: {}", d.tipo)));
    }
    Ok(cfg::DatosCargo {
        nombre: requerido("nombre", &d.nombre, 200)?,
        tipo: d.tipo,
    })
}

pub async fn listar_cargos(
    State(pool): State<PgPool>,
    user: AuthUser,
) -> ApiResult<Json<Vec<cfg::CargoConfig>>> {
    let mut tx = tx!(pool, user);
    Ok(Json(cfg::listar_cargos(&mut tx).await?))
}

pub async fn crear_cargo(
    State(pool): State<PgPool>,
    user: AuthUser,
    Json(d): Json<cfg::DatosCargo>,
) -> ApiResult<(StatusCode, Json<Creado>)> {
    let d = validar_cargo(d)?;
    let mut tx = tx!(pool, user);
    let id = cfg::crear_cargo(&mut tx, &d).await.map_err(error)?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(Creado { id })))
}

pub async fn actualizar_cargo(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(d): Json<cfg::DatosCargo>,
) -> ApiResult<StatusCode> {
    let d = validar_cargo(d)?;
    let mut tx = tx!(pool, user);
    let r = una_fila(
        cfg::actualizar_cargo(&mut tx, id, &d)
            .await
            .map_err(error)?,
    )?;
    tx.commit().await?;
    Ok(r)
}

pub async fn eliminar_cargo(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let mut tx = tx!(pool, user);
    let r = una_fila(cfg::eliminar_cargo(&mut tx, id).await.map_err(error)?)?;
    tx.commit().await?;
    Ok(r)
}

// --- Contratos -------------------------------------------------------------------------

fn validar_contrato(d: cfg::DatosContrato) -> Result<cfg::DatosContrato, ApiError> {
    let unidad = d.duracion_unidad.filter(|u| !u.is_empty());
    let hay_duracion = d.duracion_valor.is_some() || unidad.is_some();
    if d.fecha_fin.is_some() && hay_duracion {
        return Err(invalido("elegí fecha de fin o duración, no ambas"));
    }
    if d.fecha_inicio.is_none() {
        return Err(invalido("la fecha de inicio es obligatoria"));
    }
    if d.fecha_fin.is_none() {
        if !d.duracion_valor.is_some_and(|v| v > 0) {
            return Err(invalido("indicá la fecha de fin o una duración positiva"));
        }
        if !unidad
            .as_deref()
            .is_some_and(|u| UNIDADES_DURACION.contains(&u))
        {
            return Err(invalido("la unidad de duración debe ser meses o dias"));
        }
    }
    // Tiempo de renovación: opcional; si se carga, valor positivo + unidad.
    let renov_unidad = d.tiempo_renovacion_unidad.filter(|u| !u.is_empty());
    match (d.tiempo_renovacion_valor, renov_unidad.as_deref()) {
        (None, None) => {}
        (Some(v), Some(u)) if v > 0 && UNIDADES_DURACION.contains(&u) => {}
        _ => {
            return Err(invalido(
                "tiempo de renovación: indicá un número positivo y la unidad (meses o dias), o dejalo vacío",
            ));
        }
    }
    Ok(cfg::DatosContrato {
        codigo: requerido("código", &d.codigo, 60)?,
        nombre: opcional("nombre", &d.nombre, 200)?,
        duracion_unidad: unidad,
        tiempo_renovacion_unidad: renov_unidad,
        ..d
    })
}

pub async fn listar_contratos(
    State(pool): State<PgPool>,
    user: AuthUser,
) -> ApiResult<Json<Vec<cfg::Contrato>>> {
    let mut tx = tx!(pool, user);
    Ok(Json(cfg::listar_contratos(&mut tx).await?))
}

pub async fn crear_contrato(
    State(pool): State<PgPool>,
    user: AuthUser,
    Json(d): Json<cfg::DatosContrato>,
) -> ApiResult<(StatusCode, Json<Creado>)> {
    let d = validar_contrato(d)?;
    let mut tx = tx!(pool, user);
    let id = cfg::crear_contrato(&mut tx, &d).await.map_err(error)?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(Creado { id })))
}

pub async fn actualizar_contrato(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(d): Json<cfg::DatosContrato>,
) -> ApiResult<StatusCode> {
    let d = validar_contrato(d)?;
    let mut tx = tx!(pool, user);
    let r = una_fila(
        cfg::actualizar_contrato(&mut tx, id, &d)
            .await
            .map_err(error)?,
    )?;
    tx.commit().await?;
    Ok(r)
}

pub async fn eliminar_contrato(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let mut tx = tx!(pool, user);
    let r = una_fila(cfg::eliminar_contrato(&mut tx, id).await.map_err(error)?)?;
    tx.commit().await?;
    Ok(r)
}

// --- Composición salarial y métricas por cargo ---------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NuevaComposicion {
    pub servicio_id: Uuid,
    pub cargo_id: Uuid,
    pub haber_basico_diurno: Decimal,
    pub haber_basico_nocturno: Option<Decimal>,
    #[serde(default)]
    pub cantidad_mujeres: Option<i32>,
    #[serde(default)]
    pub cantidad_hombres: Option<i32>,
    #[serde(default)]
    pub tiempo_capacitacion_dias: Option<i32>,
    #[serde(default)]
    pub tiempo_ojt_dias: Option<i32>,
    #[serde(default)]
    pub tiempo_capacitacion_ajustada_dias: Option<i32>,
}

fn validar_montos(d: cfg::DatosComposicion) -> Result<cfg::DatosComposicion, ApiError> {
    for (campo, v) in [
        ("cantidad de mujeres", d.cantidad_mujeres),
        ("cantidad de hombres", d.cantidad_hombres),
    ] {
        if v.is_some_and(|n| n < 0) {
            return Err(invalido(format!("{campo}: no puede ser negativa")));
        }
    }
    for (campo, v) in [
        ("tiempo de capacitación", d.tiempo_capacitacion_dias),
        ("tiempo de OJT", d.tiempo_ojt_dias),
        (
            "tiempo de capacitación ajustada",
            d.tiempo_capacitacion_ajustada_dias,
        ),
    ] {
        if v.is_some_and(|n| !(0..=3650).contains(&n)) {
            return Err(invalido(format!("{campo}: entre 0 y 3650 días")));
        }
    }
    Ok(cfg::DatosComposicion {
        haber_basico_diurno: no_negativo("haber básico diurno", d.haber_basico_diurno)?,
        haber_basico_nocturno: d
            .haber_basico_nocturno
            .map(|v| no_negativo("haber básico nocturno", v))
            .transpose()?,
        ..d
    })
}

pub async fn crear_composicion(
    State(pool): State<PgPool>,
    user: AuthUser,
    Json(b): Json<NuevaComposicion>,
) -> ApiResult<(StatusCode, Json<Creado>)> {
    let d = validar_montos(cfg::DatosComposicion {
        haber_basico_diurno: b.haber_basico_diurno,
        haber_basico_nocturno: b.haber_basico_nocturno,
        cantidad_mujeres: b.cantidad_mujeres,
        cantidad_hombres: b.cantidad_hombres,
        tiempo_capacitacion_dias: b.tiempo_capacitacion_dias,
        tiempo_ojt_dias: b.tiempo_ojt_dias,
        tiempo_capacitacion_ajustada_dias: b.tiempo_capacitacion_ajustada_dias,
    })?;
    let mut tx = tx!(pool, user);
    let id = cfg::crear_composicion(&mut tx, b.servicio_id, b.cargo_id, &d)
        .await
        .map_err(|e| {
            if e.es_duplicado() {
                ApiError::Conflict("ese cargo ya tiene composición en ese servicio: editala".into())
            } else {
                error(e)
            }
        })?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(Creado { id })))
}

pub async fn actualizar_composicion(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(d): Json<cfg::DatosComposicion>,
) -> ApiResult<StatusCode> {
    let d = validar_montos(d)?;
    let mut tx = tx!(pool, user);
    let r = una_fila(
        cfg::actualizar_composicion(&mut tx, id, &d)
            .await
            .map_err(error)?,
    )?;
    tx.commit().await?;
    Ok(r)
}

pub async fn metricas_de(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<cfg::MetricaAsignada>>> {
    let mut tx = tx!(pool, user);
    Ok(Json(cfg::metricas_de(&mut tx, id).await?))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NuevaMetrica {
    pub metrica_id: Uuid,
    /// TODO(negocio): escala/uso de `peso_pct` sin confirmar; solo se exige que no sea negativo.
    pub peso_pct: Decimal,
    pub monto_bono: Decimal,
}

pub async fn asignar_metrica(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(b): Json<NuevaMetrica>,
) -> ApiResult<(StatusCode, Json<Creado>)> {
    let peso = no_negativo("peso", b.peso_pct)?;
    let monto = no_negativo("monto del bono", b.monto_bono)?;
    let mut tx = tx!(pool, user);
    let nueva = cfg::asignar_metrica(&mut tx, id, b.metrica_id, peso, monto)
        .await
        .map_err(|e| {
            if e.es_duplicado() {
                ApiError::Conflict("esa métrica ya está asignada a este cargo".into())
            } else {
                error(e)
            }
        })?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(Creado { id: nueva })))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdicionMetrica {
    pub peso_pct: Decimal,
    pub monto_bono: Decimal,
    #[serde(default)]
    pub umbrales: Vec<cfg::UmbralConfig>,
}

fn validar_umbrales(v: Vec<cfg::UmbralConfig>) -> Result<Vec<cfg::UmbralConfig>, ApiError> {
    if v.len() > 50 {
        return Err(invalido("como máximo 50 tramos"));
    }
    v.into_iter()
        .enumerate()
        .map(|(i, u)| {
            if let (Some(a), Some(b)) = (u.intervalo_min, u.intervalo_max)
                && a > b
            {
                return Err(invalido(format!(
                    "tramo {}: el mínimo supera al máximo",
                    i + 1
                )));
            }
            Ok(cfg::UmbralConfig {
                pct_bono: no_negativo("% del bono", u.pct_bono)?,
                observaciones: opcional("observaciones", &u.observaciones, 500)?,
                ..u
            })
        })
        .collect()
}

pub async fn actualizar_metrica(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(b): Json<EdicionMetrica>,
) -> ApiResult<StatusCode> {
    let peso = no_negativo("peso", b.peso_pct)?;
    let monto = no_negativo("monto del bono", b.monto_bono)?;
    let umbrales = validar_umbrales(b.umbrales)?;
    let mut tx = tx!(pool, user);
    let r = una_fila(
        cfg::actualizar_metrica(&mut tx, id, peso, monto, &umbrales)
            .await
            .map_err(error)?,
    )?;
    tx.commit().await?;
    Ok(r)
}

pub async fn quitar_metrica(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let mut tx = tx!(pool, user);
    let r = una_fila(cfg::quitar_metrica(&mut tx, id).await.map_err(error)?)?;
    tx.commit().await?;
    Ok(r)
}

// --- Catálogos de métricas y de bonos -------------------------------------------------------

pub async fn listar_metricas_catalogo(
    State(pool): State<PgPool>,
    user: AuthUser,
) -> ApiResult<Json<Vec<cfg::MetricaCatalogo>>> {
    let mut tx = tx!(pool, user);
    Ok(Json(cfg::listar_metricas_catalogo(&mut tx).await?))
}

pub async fn crear_metrica_catalogo(
    State(pool): State<PgPool>,
    user: AuthUser,
    Json(d): Json<cfg::DatosMetricaCatalogo>,
) -> ApiResult<(StatusCode, Json<Creado>)> {
    let d = cfg::DatosMetricaCatalogo {
        nombre: requerido("nombre", &d.nombre, 200)?,
        concepto: opcional("concepto", &d.concepto, 2000)?,
        formula: opcional("fórmula", &d.formula, 2000)?,
    };
    let mut tx = tx!(pool, user);
    let id = cfg::crear_metrica_catalogo(&mut tx, &d)
        .await
        .map_err(error)?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(Creado { id })))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatosBono {
    pub nombre: String,
}

pub async fn listar_bonos(
    State(pool): State<PgPool>,
    user: AuthUser,
) -> ApiResult<Json<Vec<cfg::BonoCatalogo>>> {
    let mut tx = tx!(pool, user);
    Ok(Json(cfg::listar_bonos(&mut tx).await?))
}

pub async fn crear_bono(
    State(pool): State<PgPool>,
    user: AuthUser,
    Json(d): Json<DatosBono>,
) -> ApiResult<(StatusCode, Json<Creado>)> {
    let nombre = requerido("nombre", &d.nombre, 200)?;
    let mut tx = tx!(pool, user);
    let id = cfg::crear_bono(&mut tx, &nombre).await.map_err(error)?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(Creado { id })))
}

pub async fn actualizar_bono(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(d): Json<DatosBono>,
) -> ApiResult<StatusCode> {
    let nombre = requerido("nombre", &d.nombre, 200)?;
    let mut tx = tx!(pool, user);
    let r = una_fila(
        cfg::actualizar_bono(&mut tx, id, &nombre)
            .await
            .map_err(error)?,
    )?;
    tx.commit().await?;
    Ok(r)
}

// --- Composición compartida (cargo en varios centros de costo) ------------------------------

#[derive(Debug, Serialize)]
pub struct MontoCentroDto {
    pub centro_costo_id: Uuid,
    pub centro_costo_nombre: String,
    pub monto: Decimal,
    /// % sobre el total (2 decimales).
    pub porcentaje: Decimal,
}

#[derive(Debug, Serialize)]
pub struct ComposicionCompartidaDto {
    pub cargo_id: Uuid,
    /// Solo en el listado.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cargo_nombre: Option<String>,
    /// Centros de los servicios activos donde está el cargo. Con menos de 2, no se puede guardar.
    pub centros_posibles: Vec<cfg::CentroRef>,
    pub compartido: bool,
    pub distribucion: Vec<MontoCentroDto>,
    pub total: Decimal,
}

async fn armar_compartida(
    tx: &mut sqlx::PgConnection,
    cargo_id: Uuid,
) -> ApiResult<ComposicionCompartidaDto> {
    use domain::composicion_compartida::{self as cc, MontoCentro};
    let posibles = cfg::centros_posibles(tx, cargo_id).await?;
    let filas = cfg::distribucion(tx, cargo_id).await?;
    let pares: Vec<MontoCentro> = filas
        .iter()
        .map(|f| MontoCentro {
            centro_costo_id: f.centro_costo_id,
            monto: f.monto,
        })
        .collect();
    let pct = cc::porcentajes(&pares);
    Ok(ComposicionCompartidaDto {
        cargo_id,
        cargo_nombre: None,
        compartido: posibles.len() >= cc::MIN_CENTROS,
        centros_posibles: posibles,
        total: cc::total(&pares),
        distribucion: filas
            .into_iter()
            .zip(pct)
            .map(|(f, porcentaje)| MontoCentroDto {
                centro_costo_id: f.centro_costo_id,
                centro_costo_nombre: f.centro_costo_nombre,
                monto: f.monto,
                porcentaje,
            })
            .collect(),
    })
}

/// GET /config/composicion-compartida → cargos con distribución cargada.
pub async fn listar_compartidas(
    State(pool): State<PgPool>,
    user: AuthUser,
) -> ApiResult<Json<Vec<ComposicionCompartidaDto>>> {
    let mut tx = tx!(pool, user);
    let cargos = cfg::cargos_con_distribucion(&mut tx).await?;
    let mut v = Vec::with_capacity(cargos.len());
    for c in cargos {
        let mut d = armar_compartida(&mut tx, c.id).await?;
        d.cargo_nombre = Some(c.nombre);
        v.push(d);
    }
    Ok(Json(v))
}

pub async fn obtener_compartida(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(cargo_id): Path<Uuid>,
) -> ApiResult<Json<ComposicionCompartidaDto>> {
    let mut tx = tx!(pool, user);
    Ok(Json(armar_compartida(&mut tx, cargo_id).await?))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParMontoCentro {
    pub centro_costo_id: Uuid,
    pub monto: Decimal,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuardarCompartida {
    /// Vacío = quitar la distribución del cargo.
    pub centros: Vec<ParMontoCentro>,
}

/// PUT /config/composicion-compartida/{cargo_id}: reemplaza la distribución completa.
pub async fn guardar_compartida(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(cargo_id): Path<Uuid>,
    Json(b): Json<GuardarCompartida>,
) -> ApiResult<Json<ComposicionCompartidaDto>> {
    use domain::composicion_compartida::{self as cc, MontoCentro};
    let pares: Vec<MontoCentro> = b
        .centros
        .iter()
        .map(|p| MontoCentro {
            centro_costo_id: p.centro_costo_id,
            monto: p.monto,
        })
        .collect();
    let mut tx = tx!(pool, user);
    if !pares.is_empty() {
        let posibles: Vec<Uuid> = cfg::centros_posibles(&mut tx, cargo_id)
            .await?
            .into_iter()
            .map(|c| c.id)
            .collect();
        cc::validar(&posibles, &pares).map_err(|e| invalido(e.mensaje()))?;
    }
    cfg::guardar_distribucion(&mut tx, cargo_id, &pares)
        .await
        .map_err(error)?;
    let dto = armar_compartida(&mut tx, cargo_id).await?;
    tx.commit().await?;
    Ok(Json(dto))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn contrato(fin: bool, valor: Option<i32>, unidad: Option<&str>) -> cfg::DatosContrato {
        cfg::DatosContrato {
            codigo: " K-1 ".into(),
            nombre: Some(" ".into()),
            cliente_id: None,
            fecha_inicio: NaiveDate::from_ymd_opt(2026, 1, 1),
            fecha_fin: fin.then(|| NaiveDate::from_ymd_opt(2026, 12, 31).unwrap()),
            duracion_valor: valor,
            duracion_unidad: unidad.map(Into::into),
            tiempo_renovacion_valor: None,
            tiempo_renovacion_unidad: None,
        }
    }

    #[test]
    fn contrato_tiempo_de_renovacion_opcional() {
        let con = |v: Option<i32>, u: Option<&str>| cfg::DatosContrato {
            tiempo_renovacion_valor: v,
            tiempo_renovacion_unidad: u.map(Into::into),
            ..contrato(true, None, None)
        };
        assert!(validar_contrato(con(None, None)).is_ok());
        assert!(validar_contrato(con(None, Some(""))).is_ok());
        assert!(validar_contrato(con(Some(3), Some("meses"))).is_ok());
        assert!(validar_contrato(con(Some(3), None)).is_err());
        assert!(validar_contrato(con(Some(0), Some("meses"))).is_err());
        assert!(validar_contrato(con(Some(3), Some("años"))).is_err());
    }

    #[test]
    fn contrato_fin_o_duracion_nunca_ambas() {
        let ok = validar_contrato(contrato(true, None, None)).unwrap();
        assert_eq!((ok.codigo.as_str(), ok.nombre), ("K-1", None));
        assert!(validar_contrato(contrato(false, Some(12), Some("meses"))).is_ok());
        assert!(validar_contrato(contrato(true, Some(12), Some("meses"))).is_err());
        assert!(validar_contrato(contrato(false, None, None)).is_err());
        assert!(validar_contrato(contrato(false, Some(12), Some("años"))).is_err());
    }

    #[test]
    fn servicio_normaliza_dias_y_valida_catalogos() {
        let base = serde_json::json!({
            "codigo": "", "nombre": " Retención ", "centro_costo_id": null, "contrato_comercial_id": null,
            "cliente_id": null, "proyecto_id": null, "tipo_contrato": null, "modalidad": "Presencial",
            "tipo_turno": "", "dias_trabajo": ["Viernes", "Lunes", "Lunes"], "departamento_region": null,
            "manual_funciones_codigo": null, "tiempo_capacitacion_dias": 5, "capacitacion_pagada": false,
            "tiene_refrigerio": true, "activo": true
        });
        let d = validar_servicio(serde_json::from_value(base.clone()).unwrap()).unwrap();
        assert_eq!(
            (d.codigo, d.nombre.as_str(), d.tipo_turno),
            (None, "Retención", None)
        );
        assert_eq!(d.dias_trabajo, ["Lunes", "Viernes"]);
        let mut malo = base.clone();
        malo["modalidad"] = "A distancia".into();
        assert!(validar_servicio(serde_json::from_value(malo).unwrap()).is_err());
        let mut malo = base;
        malo["dias_trabajo"] = serde_json::json!(["Feriado"]);
        assert!(validar_servicio(serde_json::from_value(malo).unwrap()).is_err());
    }

    #[test]
    fn tramos_validos() {
        let t = |a: i64, b: i64| cfg::UmbralConfig {
            intervalo_min: Some(Decimal::from(a)),
            intervalo_max: Some(Decimal::from(b)),
            pct_bono: Decimal::from(100),
            observaciones: Some("  ".into()),
            orden: 1,
        };
        assert_eq!(
            validar_umbrales(vec![t(0, 80)]).unwrap()[0].observaciones,
            None
        );
        assert!(validar_umbrales(vec![t(90, 80)]).is_err());
    }
}

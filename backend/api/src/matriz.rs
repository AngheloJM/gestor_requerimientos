//! GET /matriz-servicio?servicio_id=&cliente_id=&centro_costo_id=&proyecto_id=
//! Orden: Centro de Costo → Cliente → Servicio → Cargo. Visibilidad: RLS.

use axum::{
    Json,
    extract::{Query, State},
};
use db::PgPool;
use domain::{FilaMatriz, FiltrosMatriz, MetricaBono, Umbral};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    dto::{RefDto, monto},
    error::{ApiResult, tx_usuario},
};

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatrizQuery {
    pub servicio_id: Option<Uuid>,
    pub cliente_id: Option<Uuid>,
    pub centro_costo_id: Option<Uuid>,
    pub proyecto_id: Option<Uuid>,
}

impl From<MatrizQuery> for FiltrosMatriz {
    fn from(q: MatrizQuery) -> Self {
        Self {
            servicio_id: q.servicio_id.map(Into::into),
            cliente_id: q.cliente_id.map(Into::into),
            centro_costo_id: q.centro_costo_id.map(Into::into),
            proyecto_id: q.proyecto_id.map(Into::into),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CentroCostoDto {
    pub id: Uuid,
    pub codigo: String,
    pub nombre: String,
}

#[derive(Debug, Serialize)]
pub struct ServicioDto {
    pub id: Uuid,
    pub codigo: Option<String>,
    pub nombre: String,
}

#[derive(Debug, Serialize)]
pub struct CargoDto {
    pub id: Uuid,
    pub nombre: String,
    pub tipo: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UmbralDto {
    pub intervalo_min: Option<Decimal>,
    pub intervalo_max: Option<Decimal>,
    pub pct_bono: Decimal,
    pub observaciones: Option<String>,
    pub orden: i32,
}

#[derive(Debug, Serialize)]
pub struct MetricaDto {
    pub id: Uuid,
    pub metrica_id: Uuid,
    pub nombre: String,
    pub concepto: Option<String>,
    pub formula: Option<String>,
    pub peso_pct: Decimal,
    pub monto_bono: Decimal,
    pub umbrales: Vec<UmbralDto>,
}

/// Montos como string con 2 decimales. Porcentajes e intervalos, tal cual la base.
#[derive(Debug, Serialize)]
pub struct FilaMatrizDto {
    pub centro_costo: Option<CentroCostoDto>,
    pub cliente: Option<RefDto>,
    pub proyecto: Option<RefDto>,
    pub servicio: ServicioDto,
    pub cargo: CargoDto,
    pub servicio_cargo_id: Uuid,
    pub haber_basico_diurno: Decimal,
    pub haber_basico_nocturno: Option<Decimal>,
    /// Diurno + nocturno (nocturno NULL cuenta como 0).
    pub total_haberes: Decimal,
    /// TODO(negocio): `null` hasta que exista el resultado mensual de cada métrica
    /// (necesario para saber qué bonos aplican).
    pub total_ganado: Option<Decimal>,
    pub metricas: Vec<MetricaDto>,
}

impl From<&Umbral> for UmbralDto {
    fn from(u: &Umbral) -> Self {
        Self {
            intervalo_min: u.intervalo_min,
            intervalo_max: u.intervalo_max,
            pct_bono: u.pct_bono,
            observaciones: u.observaciones.clone(),
            orden: u.orden,
        }
    }
}

impl From<&MetricaBono> for MetricaDto {
    fn from(m: &MetricaBono) -> Self {
        Self {
            id: m.id.into(),
            metrica_id: m.metrica_id.into(),
            nombre: m.nombre.clone(),
            concepto: m.concepto.clone(),
            formula: m.formula.clone(),
            peso_pct: m.peso_pct,
            monto_bono: monto(m.monto_bono),
            umbrales: m.umbrales.iter().map(UmbralDto::from).collect(),
        }
    }
}

impl From<FilaMatriz> for FilaMatrizDto {
    fn from(f: FilaMatriz) -> Self {
        let c = &f.composicion;
        Self {
            servicio_cargo_id: c.id.into(),
            haber_basico_diurno: monto(c.haber_basico_diurno),
            haber_basico_nocturno: c.haber_basico_nocturno.map(monto),
            total_haberes: monto(c.total_haberes()),
            total_ganado: c.total_ganado().map(monto),
            metricas: c.metricas.iter().map(MetricaDto::from).collect(),
            centro_costo: f.centro_costo.map(|cc| CentroCostoDto {
                id: cc.id.into(),
                codigo: cc.codigo,
                nombre: cc.nombre,
            }),
            cliente: f.cliente.map(|cl| RefDto::new(cl.id, Some(cl.nombre))),
            proyecto: f.proyecto.map(|p| RefDto::new(p.id, Some(p.nombre))),
            servicio: ServicioDto {
                id: f.servicio.id.into(),
                codigo: f.servicio.codigo,
                nombre: f.servicio.nombre,
            },
            cargo: CargoDto {
                id: f.cargo.id.into(),
                nombre: f.cargo.nombre,
                tipo: f.cargo.tipo,
            },
        }
    }
}

/// TODO(negocio): si `estandar` debe ver montos salariales o solo la estructura. RLS filtra
/// filas, no columnas: ocultar montos requeriría una vista o privilegios por columna.
pub async fn get_matriz_servicio(
    State(pool): State<PgPool>,
    user: AuthUser,
    Query(q): Query<MatrizQuery>,
) -> ApiResult<Json<Vec<FilaMatrizDto>>> {
    let mut tx = tx_usuario(&pool, &user).await?;
    let filas = db::matriz::listar_matriz(&mut tx, q.into()).await?;
    Ok(Json(filas.into_iter().map(FilaMatrizDto::from).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{
        Cargo, CargoId, ComposicionSalarial, MetricaId, Servicio, ServicioCargoId,
        ServicioCargoMetricaId, ServicioId,
    };
    use rust_decimal_macros::dec;
    use serde_json::json;

    #[test]
    fn dto_montos_total_ganado_pendiente_y_metricas() {
        let sid = ServicioId::new();
        let cid = CargoId::new();
        let fila = FilaMatriz {
            centro_costo: None,
            cliente: None,
            proyecto: None,
            servicio: Servicio {
                id: sid,
                codigo: None,
                nombre: "Call center".into(),
                centro_costo_id: None,
                contrato_comercial_id: None,
                cliente_id: None,
                proyecto_id: None,
                activo: true,
            },
            cargo: Cargo {
                id: cid,
                nombre: "Operador".into(),
                tipo: None,
            },
            composicion: ComposicionSalarial {
                id: ServicioCargoId::new(),
                servicio_id: sid,
                cargo_id: cid,
                haber_basico_diurno: dec!(3000),
                haber_basico_nocturno: None,
                metricas: vec![MetricaBono {
                    id: ServicioCargoMetricaId::new(),
                    metrica_id: MetricaId::new(),
                    nombre: "AHT".into(),
                    concepto: None,
                    formula: None,
                    peso_pct: dec!(60),
                    monto_bono: dec!(200),
                    umbrales: vec![],
                }],
            },
        };
        let v = serde_json::to_value(FilaMatrizDto::from(fila)).unwrap();
        assert_eq!(v["haber_basico_diurno"], json!("3000.00"));
        assert_eq!(v["haber_basico_nocturno"], json!(null));
        assert_eq!(v["total_haberes"], json!("3000.00"));
        assert_eq!(v["total_ganado"], json!(null));
        assert_eq!(v["centro_costo"], json!(null));
        assert_eq!(v["metricas"][0]["monto_bono"], json!("200.00"));
        assert_eq!(v["metricas"][0]["peso_pct"], json!("60"));
    }
}

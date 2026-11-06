//! Composición Salarial = tabla puente `servicio_cargos` (una fila por Servicio+Cargo), con
//! sus métricas de bono (`servicio_cargo_metricas`) y escalas (`servicio_cargo_metrica_umbrales`).

use rust_decimal::Decimal;

use crate::{CargoId, MetricaId, ServicioCargoId, ServicioCargoMetricaId, ServicioId};

/// Escala de una métrica: si el cumplimiento cae en `[intervalo_min, intervalo_max]` se paga
/// `pct_bono` del `monto_bono`. Un extremo `None` se trata como no acotado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Umbral {
    pub intervalo_min: Option<Decimal>,
    pub intervalo_max: Option<Decimal>,
    pub pct_bono: Decimal,
    pub observaciones: Option<String>,
    pub orden: i32,
}

impl Umbral {
    pub fn contiene(&self, cumplimiento: Decimal) -> bool {
        self.intervalo_min.is_none_or(|m| cumplimiento >= m)
            && self.intervalo_max.is_none_or(|m| cumplimiento <= m)
    }
}

/// Métrica de bono asociada a un servicio+cargo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricaBono {
    pub id: ServicioCargoMetricaId,
    pub metrica_id: MetricaId,
    pub nombre: String,
    pub concepto: Option<String>,
    pub formula: Option<String>,
    pub peso_pct: Decimal,
    pub monto_bono: Decimal,
    /// Ordenados por `orden`.
    pub umbrales: Vec<Umbral>,
}

impl MetricaBono {
    /// Primer umbral (según `orden`) cuyo intervalo contiene el cumplimiento.
    ///
    /// TODO(negocio): el monto a pagar (`pct_bono` × `monto_bono`) no se calcula todavía:
    /// - no existe la tabla del resultado real mensual de cada métrica (quién/cómo lo carga);
    /// - no está confirmada la escala de `pct_bono` (0–100 o 0–1).
    pub fn umbral_para(&self, cumplimiento: Decimal) -> Option<&Umbral> {
        self.umbrales.iter().find(|u| u.contiene(cumplimiento))
    }
}

/// Fila de `servicio_cargos`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposicionSalarial {
    pub id: ServicioCargoId,
    /// `servicio_cargos.servicio_id` es nullable en la tabla; en la matriz solo se incluyen
    /// filas con servicio. TODO(negocio): qué representan las filas con `servicio_id` NULL.
    pub servicio_id: ServicioId,
    pub cargo_id: CargoId,
    pub haber_basico_diurno: Decimal,
    pub haber_basico_nocturno: Option<Decimal>,
    /// A confirmar: cantidades por cargo del servicio (migración 0004).
    pub cantidad_mujeres: Option<i32>,
    pub cantidad_hombres: Option<i32>,
    pub metricas: Vec<MetricaBono>,
}

impl ComposicionSalarial {
    /// Haber básico diurno + nocturno (nocturno NULL se suma como 0).
    pub fn total_haberes(&self) -> Decimal {
        self.haber_basico_diurno + self.haber_basico_nocturno.unwrap_or_default()
    }

    /// Total ganado = haber básico diurno + nocturno + suma de `monto_bono` de las métricas
    /// configuradas (regla confirmada: es el cálculo que ya usa la aplicación original).
    ///
    /// Debe coincidir con el valor que muestra hoy la aplicación original (evitar regresión).
    // TODO(negocio): no ajusta por cumplimiento real mensual — revisar cuando exista esa tabla
    // (ver `MetricaBono::umbral_para`).
    pub fn total_ganado(&self) -> Decimal {
        self.total_haberes() + self.metricas.iter().map(|m| m.monto_bono).sum::<Decimal>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn umbral(min: Option<Decimal>, max: Option<Decimal>, orden: i32) -> Umbral {
        Umbral {
            intervalo_min: min,
            intervalo_max: max,
            pct_bono: dec!(50),
            observaciones: None,
            orden,
        }
    }

    #[test]
    fn umbral_intervalo_cerrado_y_extremos_abiertos() {
        let u = umbral(Some(dec!(80)), Some(dec!(90)), 1);
        assert!(!u.contiene(dec!(79.99)));
        assert!(u.contiene(dec!(80)));
        assert!(u.contiene(dec!(90)));
        assert!(!u.contiene(dec!(90.01)));
        assert!(umbral(None, Some(dec!(10)), 1).contiene(dec!(-5)));
        assert!(umbral(Some(dec!(95)), None, 1).contiene(dec!(1000)));
    }

    #[test]
    fn umbral_para_toma_el_primero_por_orden() {
        let m = MetricaBono {
            id: ServicioCargoMetricaId::new(),
            metrica_id: MetricaId::new(),
            nombre: "AHT".into(),
            concepto: None,
            formula: None,
            peso_pct: dec!(30),
            monto_bono: dec!(200),
            umbrales: vec![
                umbral(Some(dec!(0)), Some(dec!(79.99)), 1),
                umbral(Some(dec!(80)), None, 2),
            ],
        };
        assert_eq!(m.umbral_para(dec!(85)).map(|u| u.orden), Some(2));
        assert_eq!(m.umbral_para(dec!(10)).map(|u| u.orden), Some(1));
        assert_eq!(m.umbral_para(dec!(-1)), None);
    }

    #[test]
    fn totales() {
        let c = ComposicionSalarial {
            id: ServicioCargoId::new(),
            servicio_id: ServicioId::new(),
            cargo_id: CargoId::new(),
            haber_basico_diurno: dec!(3000.00),
            haber_basico_nocturno: None,
            cantidad_mujeres: None,
            cantidad_hombres: None,
            metricas: vec![],
        };
        assert_eq!(c.total_haberes(), dec!(3000.00));
        assert_eq!(
            c.total_ganado(),
            dec!(3000.00),
            "sin métricas = solo haberes"
        );
        let bono = |monto| MetricaBono {
            id: ServicioCargoMetricaId::new(),
            metrica_id: MetricaId::new(),
            nombre: "Bono".into(),
            concepto: None,
            formula: None,
            peso_pct: dec!(50),
            monto_bono: monto,
            umbrales: vec![],
        };
        let c = ComposicionSalarial {
            haber_basico_nocturno: Some(dec!(500.50)),
            cantidad_mujeres: None,
            cantidad_hombres: None,
            metricas: vec![bono(dec!(200)), bono(dec!(100))],
            ..c
        };
        assert_eq!(c.total_haberes(), dec!(3500.50));
        assert_eq!(c.total_ganado(), dec!(3800.50));
        // Caso real verificado en la aplicación original: 2800 + 0 + bono 150 = 2950.
        let real = ComposicionSalarial {
            haber_basico_diurno: dec!(2800),
            haber_basico_nocturno: Some(dec!(0)),
            cantidad_mujeres: None,
            cantidad_hombres: None,
            metricas: vec![bono(dec!(150))],
            ..c
        };
        assert_eq!(real.total_ganado(), dec!(2950));
    }
}

//! Plazos de un requerimiento nuevo (revisión del 06/11/2026, puntos 10 y 11).
//!
//! A partir de la fecha de inicio de operación (la única que escribe el usuario):
//! - **Fecha máxima de capacitación (normal)** = operación − `tiempo_capacitacion_dias` del
//!   cargo en el servicio. Si el cargo no lo tiene cargado, se usa el "tiempo de capacitación
//!   general" del servicio (el mismo dato que usa la base al crear el requerimiento).
//! - **Fecha máxima de capacitación ajustada** = operación − `tiempo_capacitacion_ajustada_dias`
//!   del cargo (sin respaldo en el servicio).
//! - **Tiempo de reclutamiento** = días hábiles desde hoy hasta la fecha de entrega a
//!   capacitación, sin sábados, domingos ni feriados.
//!
//! Valores por defecto (decisión del 2026-11-07; ver docs/PENDIENTES.md):
//! - TODO(negocio): la resta de capacitación se hace en días **corridos**.
//! - TODO(negocio): la entrega a capacitación es la **ajustada** si existe; si no, la normal.
//! - TODO(negocio): los días hábiles cuentan desde hoy (incluido) hasta la entrega (excluida),
//!   como una resta de fechas.
//! - El OJT no entra en el cálculo.

use std::collections::HashSet;

use chrono::{Datelike, Days, NaiveDate, Weekday};

/// Tiempos configurados (en días) para el servicio y cargo elegidos.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TiemposCapacitacion {
    /// `servicio_cargos.tiempo_capacitacion_dias`.
    pub capacitacion_cargo: Option<i32>,
    /// `servicio_cargos.tiempo_capacitacion_ajustada_dias`.
    pub ajustada_cargo: Option<i32>,
    /// `servicios.tiempo_capacitacion_dias` (respaldo de la normal).
    pub general_servicio: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrigenNormal {
    Cargo,
    Servicio,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plazos {
    pub capacitacion_normal: Option<NaiveDate>,
    /// De dónde salió el tiempo de la normal (`None` si no hay dato).
    pub origen_normal: Option<OrigenNormal>,
    pub capacitacion_ajustada: Option<NaiveDate>,
    /// Fecha usada para el reclutamiento: la ajustada si existe; si no, la normal.
    pub entrega_capacitacion: Option<NaiveDate>,
    /// Días hábiles de reclutamiento (0 si la entrega es hoy o ya pasó).
    pub dias_reclutamiento: Option<u32>,
}

fn restar(fecha: NaiveDate, dias: Option<i32>) -> Option<NaiveDate> {
    let d = u64::try_from(dias?).ok()?;
    fecha.checked_sub_days(Days::new(d))
}

/// Días hábiles en `[desde, hasta)`: sin sábados, domingos ni `feriados`.
pub fn dias_habiles(desde: NaiveDate, hasta: NaiveDate, feriados: &HashSet<NaiveDate>) -> u32 {
    desde
        .iter_days()
        .take_while(|d| *d < hasta)
        .filter(|d| !matches!(d.weekday(), Weekday::Sat | Weekday::Sun) && !feriados.contains(d))
        .fold(0, |n, _| n + 1)
}

pub fn calcular(
    operacion: NaiveDate,
    tiempos: TiemposCapacitacion,
    hoy: NaiveDate,
    feriados: &HashSet<NaiveDate>,
) -> Plazos {
    let (capacitacion_normal, origen_normal) = match restar(operacion, tiempos.capacitacion_cargo) {
        Some(f) => (Some(f), Some(OrigenNormal::Cargo)),
        None => match restar(operacion, tiempos.general_servicio) {
            Some(f) => (Some(f), Some(OrigenNormal::Servicio)),
            None => (None, None),
        },
    };
    let capacitacion_ajustada = restar(operacion, tiempos.ajustada_cargo);
    let entrega_capacitacion = capacitacion_ajustada.or(capacitacion_normal);
    Plazos {
        capacitacion_normal,
        origen_normal,
        capacitacion_ajustada,
        entrega_capacitacion,
        dias_reclutamiento: entrega_capacitacion.map(|e| dias_habiles(hoy, e, feriados)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn habiles_sin_fin_de_semana_ni_feriados() {
        let sin = HashSet::new();
        // Vie 06/11/2026 → mar 10/11: vie 6, lun 9 (sáb y dom fuera; el 10 no cuenta).
        assert_eq!(dias_habiles(f("2026-11-06"), f("2026-11-10"), &sin), 2);
        let feriado = HashSet::from([f("2026-11-09")]);
        assert_eq!(dias_habiles(f("2026-11-06"), f("2026-11-10"), &feriado), 1);
        // Entrega hoy o en el pasado → 0.
        assert_eq!(dias_habiles(f("2026-11-10"), f("2026-11-10"), &sin), 0);
        assert_eq!(dias_habiles(f("2026-11-10"), f("2026-11-06"), &sin), 0);
    }

    #[test]
    fn normal_y_ajustada_desde_el_cargo() {
        let t = TiemposCapacitacion {
            capacitacion_cargo: Some(10),
            ajustada_cargo: Some(15),
            general_servicio: Some(7),
        };
        let p = calcular(f("2026-12-15"), t, f("2026-11-06"), &HashSet::new());
        assert_eq!(p.capacitacion_normal, Some(f("2026-12-05")));
        assert_eq!(p.origen_normal, Some(OrigenNormal::Cargo));
        assert_eq!(p.capacitacion_ajustada, Some(f("2026-11-30")));
        // La entrega usa la ajustada.
        assert_eq!(p.entrega_capacitacion, Some(f("2026-11-30")));
        assert_eq!(p.dias_reclutamiento, Some(16));
    }

    #[test]
    fn sin_tiempos_del_cargo_usa_el_general_y_la_normal() {
        let t = TiemposCapacitacion {
            general_servicio: Some(10),
            ..Default::default()
        };
        let p = calcular(f("2026-12-15"), t, f("2026-11-06"), &HashSet::new());
        assert_eq!(p.capacitacion_normal, Some(f("2026-12-05")));
        assert_eq!(p.origen_normal, Some(OrigenNormal::Servicio));
        assert_eq!(p.capacitacion_ajustada, None);
        assert_eq!(p.entrega_capacitacion, Some(f("2026-12-05")));
    }

    #[test]
    fn sin_ningun_tiempo_no_hay_plazos() {
        let p = calcular(
            f("2026-12-15"),
            TiemposCapacitacion::default(),
            f("2026-11-06"),
            &HashSet::new(),
        );
        assert_eq!(p.capacitacion_normal, None);
        assert_eq!(p.entrega_capacitacion, None);
        assert_eq!(p.dias_reclutamiento, None);
    }
}

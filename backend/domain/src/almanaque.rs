//! Almanaque: calendario mensual de movimientos, con las cuatro categorías de la leyenda del
//! producto (Entradas / Salidas / Entregas / Rectificaciones):
//!
//! - **Entrada**: `requerimientos.fecha_inicio_operacion` (cantidad = `cantidad_personas`).
//! - **Salida**: `requerimientos.fecha_salida` (cantidad = `cantidad_personas`).
//! - **Entrega**: `requerimiento_entregas` sin `corrige_entrega_id` (cantidad = `cantidad`).
//! - **Rectificación**: `requerimiento_entregas` con `corrige_entrega_id` (corrige otra entrega).
//!
//! Por día y categoría se agrupa por servicio+cargo sumando cantidades.

use std::collections::HashMap;

use chrono::{Datelike, NaiveDate};

use crate::{CargoId, ServicioId};

/// Mes calendario válido. Rango de años acotado para rechazar valores absurdos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Periodo {
    anio: i32,
    mes: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeriodoInvalido {
    Mes(u32),
    Anio(i32),
}

impl std::fmt::Display for PeriodoInvalido {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mes(m) => write!(f, "mes inválido: {m} (debe ser 1-12)"),
            Self::Anio(a) => write!(
                f,
                "año inválido: {a} (debe estar entre {} y {})",
                Periodo::ANIO_MIN,
                Periodo::ANIO_MAX
            ),
        }
    }
}

impl std::error::Error for PeriodoInvalido {}

impl Periodo {
    pub const ANIO_MIN: i32 = 2000;
    pub const ANIO_MAX: i32 = 2100;

    pub fn new(anio: i32, mes: u32) -> Result<Self, PeriodoInvalido> {
        if !(1..=12).contains(&mes) {
            return Err(PeriodoInvalido::Mes(mes));
        }
        if !(Self::ANIO_MIN..=Self::ANIO_MAX).contains(&anio) {
            return Err(PeriodoInvalido::Anio(anio));
        }
        Ok(Self { anio, mes })
    }

    pub fn anio(self) -> i32 {
        self.anio
    }

    pub fn mes(self) -> u32 {
        self.mes
    }

    /// Primer día del mes (inclusive).
    pub fn desde(self) -> NaiveDate {
        NaiveDate::from_ymd_opt(self.anio, self.mes, 1).expect("periodo validado")
    }

    /// Primer día del mes siguiente (exclusivo).
    pub fn hasta(self) -> NaiveDate {
        let (a, m) = if self.mes == 12 {
            (self.anio + 1, 1)
        } else {
            (self.anio, self.mes + 1)
        };
        NaiveDate::from_ymd_opt(a, m, 1).expect("periodo validado")
    }

    pub fn contiene(self, fecha: NaiveDate) -> bool {
        fecha.year() == self.anio && fecha.month() == self.mes
    }

    /// Todos los días del mes, en orden.
    pub fn dias(self) -> impl Iterator<Item = NaiveDate> {
        let hasta = self.hasta();
        self.desde().iter_days().take_while(move |d| *d < hasta)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TipoMovimiento {
    Entrada,
    Salida,
    Entrega,
    Rectificacion,
}

impl TipoMovimiento {
    pub fn desde_codigo(s: &str) -> Option<Self> {
        match s {
            "entrada" => Some(Self::Entrada),
            "salida" => Some(Self::Salida),
            "entrega" => Some(Self::Entrega),
            "rectificacion" => Some(Self::Rectificacion),
            _ => None,
        }
    }
}

/// Un evento ya clasificado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Movimiento {
    pub tipo: TipoMovimiento,
    pub fecha: NaiveDate,
    /// `None` si el requerimiento no tiene servicio (`requerimientos.servicio_id` es nullable).
    pub servicio_id: Option<ServicioId>,
    pub servicio_nombre: Option<String>,
    pub cargo_id: CargoId,
    pub cargo_nombre: Option<String>,
    /// `None` si la cantidad es NULL en la base (`cantidad_personas` es nullable).
    pub cantidad: Option<u32>,
}

/// Total de un servicio+cargo en un día y categoría.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineaAlmanaque {
    pub servicio_id: Option<ServicioId>,
    pub servicio_nombre: Option<String>,
    pub cargo_id: CargoId,
    pub cargo_nombre: Option<String>,
    /// Suma de las cantidades no nulas.
    pub cantidad: u64,
    /// Registros agrupados con cantidad NULL (no se suman: no se asume un valor).
    /// TODO(negocio): ¿un requerimiento sin `cantidad_personas` cuenta como 1 persona?
    pub registros_sin_cantidad: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiaAlmanaque {
    pub fecha: NaiveDate,
    pub entradas: Vec<LineaAlmanaque>,
    pub salidas: Vec<LineaAlmanaque>,
    pub entregas: Vec<LineaAlmanaque>,
    /// TODO(negocio): si la cantidad de una rectificación reemplaza a la de la entrega que
    /// corrige o es un delta. Hoy se listan por separado, sin netear contra la original.
    pub rectificaciones: Vec<LineaAlmanaque>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Almanaque {
    pub periodo: Periodo,
    /// Un elemento por cada día del mes (incluye días sin movimientos).
    pub dias: Vec<DiaAlmanaque>,
}

type Clave = (NaiveDate, TipoMovimiento, Option<ServicioId>, CargoId);

impl Almanaque {
    /// Agrupa los movimientos del mes. Los que caen fuera del periodo se ignoran.
    pub fn construir(periodo: Periodo, movimientos: &[Movimiento]) -> Self {
        let mut grupos: HashMap<Clave, LineaAlmanaque> = HashMap::new();
        for m in movimientos.iter().filter(|m| periodo.contiene(m.fecha)) {
            let linea = grupos
                .entry((m.fecha, m.tipo, m.servicio_id, m.cargo_id))
                .or_insert_with(|| LineaAlmanaque {
                    servicio_id: m.servicio_id,
                    servicio_nombre: m.servicio_nombre.clone(),
                    cargo_id: m.cargo_id,
                    cargo_nombre: m.cargo_nombre.clone(),
                    cantidad: 0,
                    registros_sin_cantidad: 0,
                });
            match m.cantidad {
                Some(n) => linea.cantidad += u64::from(n),
                None => linea.registros_sin_cantidad += 1,
            }
        }

        let mut por_dia: HashMap<(NaiveDate, TipoMovimiento), Vec<LineaAlmanaque>> = HashMap::new();
        for ((fecha, tipo, _, _), linea) in grupos {
            por_dia.entry((fecha, tipo)).or_default().push(linea);
        }
        for v in por_dia.values_mut() {
            v.sort_by(|a, b| {
                (
                    &a.servicio_nombre,
                    &a.cargo_nombre,
                    a.servicio_id,
                    a.cargo_id,
                )
                    .cmp(&(
                        &b.servicio_nombre,
                        &b.cargo_nombre,
                        b.servicio_id,
                        b.cargo_id,
                    ))
            });
        }

        let mut tomar = |fecha, tipo| por_dia.remove(&(fecha, tipo)).unwrap_or_default();
        let dias = periodo
            .dias()
            .map(|fecha| DiaAlmanaque {
                fecha,
                entradas: tomar(fecha, TipoMovimiento::Entrada),
                salidas: tomar(fecha, TipoMovimiento::Salida),
                entregas: tomar(fecha, TipoMovimiento::Entrega),
                rectificaciones: tomar(fecha, TipoMovimiento::Rectificacion),
            })
            .collect();

        Self { periodo, dias }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, day).unwrap()
    }

    fn nov() -> Periodo {
        Periodo::new(2026, 11).unwrap()
    }

    struct Puesto {
        s: ServicioId,
        c: CargoId,
    }

    fn mov(p: &Puesto, tipo: TipoMovimiento, fecha: NaiveDate, cant: Option<u32>) -> Movimiento {
        Movimiento {
            tipo,
            fecha,
            servicio_id: Some(p.s),
            servicio_nombre: Some("Call center".into()),
            cargo_id: p.c,
            cargo_nombre: Some("Operador".into()),
            cantidad: cant,
        }
    }

    #[test]
    fn periodo_valida_y_calcula_limites() {
        assert_eq!(Periodo::new(2026, 13), Err(PeriodoInvalido::Mes(13)));
        assert_eq!(Periodo::new(1999, 5), Err(PeriodoInvalido::Anio(1999)));
        let dic = Periodo::new(2026, 12).unwrap();
        assert_eq!(dic.hasta(), NaiveDate::from_ymd_opt(2027, 1, 1).unwrap());
        assert_eq!(Periodo::new(2028, 2).unwrap().dias().count(), 29);
    }

    #[test]
    fn cuatro_categorias_separadas_y_suma_por_servicio_cargo_dia() {
        use TipoMovimiento::*;
        let p = Puesto {
            s: ServicioId::new(),
            c: CargoId::new(),
        };
        let a = Almanaque::construir(
            nov(),
            &[
                mov(&p, Entrada, d(3), Some(3)),
                mov(&p, Entrada, d(3), Some(2)),
                mov(&p, Salida, d(3), Some(1)),
                mov(&p, Entrega, d(3), Some(4)),
                mov(&p, Rectificacion, d(3), Some(1)),
                mov(&p, Entrada, d(4), Some(7)),
            ],
        );
        let dia3 = &a.dias[2];
        assert_eq!(dia3.entradas[0].cantidad, 5);
        assert_eq!(dia3.salidas[0].cantidad, 1);
        assert_eq!(dia3.entregas[0].cantidad, 4);
        assert_eq!(dia3.rectificaciones[0].cantidad, 1);
        assert_eq!(a.dias[3].entradas[0].cantidad, 7);
        assert_eq!(a.dias.len(), 30);
    }

    #[test]
    fn cantidad_nula_no_se_suma_pero_se_cuenta() {
        let p = Puesto {
            s: ServicioId::new(),
            c: CargoId::new(),
        };
        let a = Almanaque::construir(
            nov(),
            &[
                mov(&p, TipoMovimiento::Entrada, d(5), Some(2)),
                mov(&p, TipoMovimiento::Entrada, d(5), None),
            ],
        );
        let l = &a.dias[4].entradas[0];
        assert_eq!((l.cantidad, l.registros_sin_cantidad), (2, 1));
    }

    #[test]
    fn fuera_del_mes_se_ignora() {
        let p = Puesto {
            s: ServicioId::new(),
            c: CargoId::new(),
        };
        let oct = NaiveDate::from_ymd_opt(2026, 10, 31).unwrap();
        let a = Almanaque::construir(nov(), &[mov(&p, TipoMovimiento::Entrada, oct, Some(9))]);
        assert!(a.dias.iter().all(|d| d.entradas.is_empty()));
    }
}

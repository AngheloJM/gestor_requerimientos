//! Almanaque: calendario mensual de movimientos, con las cuatro categorías de la leyenda del
//! producto (Entradas / Salidas / Entregas / Rectificaciones). Fuente: la función de producción
//! `leer_eventos_almanaque` (la misma que usa la aplicación original):
//!
//! - **Entrada** (`in`): `requerimientos.fecha_inicio_operacion` (cantidad = `cantidad_personas`).
//! - **Salida** (`out`): `requerimientos.fecha_salida` (cantidad = `cantidad_personas`).
//! - **Entrega / Rectificación**: `requerimiento_entregas.tipo`, con el estado de su decisión.
//! - No se filtra por `requerimientos.estado` (igual que la función de la base).
//!
//! Por día y categoría se agrupa por servicio + cargo + cliente (+ estado de entrega).

use std::collections::BTreeMap;

use chrono::{Datelike, NaiveDate};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TipoMovimiento {
    Entrada,
    Salida,
    Entrega,
    Rectificacion,
}

impl TipoMovimiento {
    /// Códigos de `leer_eventos_almanaque` (producción): `in` / `out` para requerimientos y
    /// `requerimiento_entregas.tipo` (`entrega` / `rectificacion`) para entregas.
    pub fn desde_codigo(s: &str) -> Option<Self> {
        match s {
            "in" => Some(Self::Entrada),
            "out" => Some(Self::Salida),
            "entrega" => Some(Self::Entrega),
            "rectificacion" => Some(Self::Rectificacion),
            _ => None,
        }
    }
}

/// Un evento tal como lo devuelve la función de la base (nombres, no ids).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Movimiento {
    pub tipo: TipoMovimiento,
    pub fecha: NaiveDate,
    pub cliente: Option<String>,
    pub servicio: Option<String>,
    pub cargo: Option<String>,
    /// `None` si la cantidad es NULL (`requerimientos.cantidad_personas` es nullable).
    /// Las rectificaciones pueden ser negativas (CHECK de la base: `cantidad <> 0`).
    pub cantidad: Option<i64>,
    pub folio: Option<String>,
    /// Solo entregas/rectificaciones: `pendiente` (sin decisión), `aceptada` o `rechazada`.
    pub estado_entrega: Option<String>,
}

/// Total de un cliente+servicio+cargo (y estado de entrega) en un día y categoría.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineaAlmanaque {
    pub cliente: Option<String>,
    pub servicio: Option<String>,
    pub cargo: Option<String>,
    pub estado_entrega: Option<String>,
    /// Suma de las cantidades no nulas.
    pub cantidad: i64,
    /// Registros agrupados con cantidad NULL (no se suman: no se asume un valor).
    /// TODO(negocio): ¿un requerimiento sin `cantidad_personas` cuenta como 1 persona?
    pub registros_sin_cantidad: u32,
    /// Folios de los requerimientos agrupados (para el detalle del día).
    pub folios: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiaAlmanaque {
    pub fecha: NaiveDate,
    pub entradas: Vec<LineaAlmanaque>,
    pub salidas: Vec<LineaAlmanaque>,
    pub entregas: Vec<LineaAlmanaque>,
    /// Se listan aparte, con su signo, sin netear contra la entrega que corrigen.
    /// TODO(negocio): confirmar que la cantidad es un ajuste (el CHECK `cantidad <> 0`, que
    /// admite negativos, sugiere que sí).
    pub rectificaciones: Vec<LineaAlmanaque>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Almanaque {
    pub periodo: Periodo,
    /// Un elemento por cada día del mes (incluye días sin movimientos).
    pub dias: Vec<DiaAlmanaque>,
}

type Clave = (
    NaiveDate,
    TipoMovimiento,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

impl Almanaque {
    /// Agrupa los movimientos del mes. Los que caen fuera del periodo se ignoran.
    pub fn construir(periodo: Periodo, movimientos: &[Movimiento]) -> Self {
        let mut grupos: BTreeMap<Clave, LineaAlmanaque> = BTreeMap::new();
        for m in movimientos.iter().filter(|m| periodo.contiene(m.fecha)) {
            let clave = (
                m.fecha,
                m.tipo,
                m.servicio.clone(),
                m.cargo.clone(),
                m.cliente.clone(),
                m.estado_entrega.clone(),
            );
            let linea = grupos.entry(clave).or_insert_with(|| LineaAlmanaque {
                cliente: m.cliente.clone(),
                servicio: m.servicio.clone(),
                cargo: m.cargo.clone(),
                estado_entrega: m.estado_entrega.clone(),
                cantidad: 0,
                registros_sin_cantidad: 0,
                folios: Vec::new(),
            });
            match m.cantidad {
                Some(n) => linea.cantidad += n,
                None => linea.registros_sin_cantidad += 1,
            }
            if let Some(f) = &m.folio
                && !linea.folios.contains(f)
            {
                linea.folios.push(f.clone());
            }
        }

        // BTreeMap: ya quedan ordenadas por fecha, tipo, servicio, cargo, cliente, estado.
        let mut por_dia: BTreeMap<(NaiveDate, TipoMovimiento), Vec<LineaAlmanaque>> =
            BTreeMap::new();
        for ((fecha, tipo, ..), mut linea) in grupos {
            linea.folios.sort();
            por_dia.entry((fecha, tipo)).or_default().push(linea);
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

    fn mov(
        tipo: TipoMovimiento,
        fecha: NaiveDate,
        servicio: &str,
        cant: Option<i64>,
        folio: &str,
    ) -> Movimiento {
        Movimiento {
            tipo,
            fecha,
            cliente: Some("Cliente A".into()),
            servicio: Some(servicio.into()),
            cargo: Some("Operador".into()),
            cantidad: cant,
            folio: Some(folio.into()),
            estado_entrega: None,
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
    fn suma_por_dia_tipo_servicio_y_cargo_con_folios() {
        use TipoMovimiento::*;
        let movs = vec![
            mov(Entrada, d(3), "Call center", Some(3), "REQ-000002"),
            mov(Entrada, d(3), "Call center", Some(4), "REQ-000001"),
            mov(Entrada, d(3), "Call center", None, "REQ-000003"),
            mov(Entrada, d(3), "Soporte", Some(1), "REQ-000004"),
            mov(Salida, d(3), "Call center", Some(2), "REQ-000001"),
            mov(Rectificacion, d(6), "Call center", Some(-1), "REQ-000001"),
            mov(
                Entrada,
                NaiveDate::from_ymd_opt(2026, 12, 1).unwrap(),
                "Call center",
                Some(9),
                "REQ-000009",
            ),
        ];
        let a = Almanaque::construir(nov(), &movs);
        assert_eq!(a.dias.len(), 30, "un elemento por día");
        let dia3 = &a.dias[2];
        assert_eq!(dia3.entradas.len(), 2);
        let cc = &dia3.entradas[0];
        assert_eq!(cc.servicio.as_deref(), Some("Call center"));
        assert_eq!((cc.cantidad, cc.registros_sin_cantidad), (7, 1));
        assert_eq!(cc.folios, ["REQ-000001", "REQ-000002", "REQ-000003"]);
        assert_eq!(dia3.salidas[0].cantidad, 2);
        assert_eq!(
            a.dias[5].rectificaciones[0].cantidad, -1,
            "rectificación con signo"
        );
        assert!(
            a.dias.iter().all(|d| d.fecha.month() == 11),
            "fuera del mes se ignora"
        );
    }

    #[test]
    fn entregas_separadas_por_estado() {
        use TipoMovimiento::*;
        let mut aceptada = mov(Entrega, d(5), "Call center", Some(3), "REQ-000001");
        aceptada.estado_entrega = Some("aceptada".into());
        let mut pendiente = mov(Entrega, d(5), "Call center", Some(2), "REQ-000002");
        pendiente.estado_entrega = Some("pendiente".into());
        let a = Almanaque::construir(nov(), &[pendiente, aceptada]);
        let estados: Vec<_> = a.dias[4]
            .entregas
            .iter()
            .map(|l| (l.estado_entrega.as_deref(), l.cantidad))
            .collect();
        assert_eq!(estados, [(Some("aceptada"), 3), (Some("pendiente"), 2)]);
    }
}

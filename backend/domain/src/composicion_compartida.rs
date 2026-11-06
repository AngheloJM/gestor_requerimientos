//! Composición salarial de un cargo **compartido entre varios centros de costo**
//! (revisión 04/11/2026): pares (centro de costo, monto) y el % de cada centro sobre el total.
//!
//! Reglas (las dos "a confirmar" en docs/PENDIENTES.md):
//! - Centros de costo *posibles* de un cargo = centros de los servicios **activos** donde el
//!   cargo está asignado (`servicio_cargos`). Con menos de dos, el cargo no es compartido y su
//!   composición se arma dentro del servicio: no se puede guardar acá.
//! - La distribución necesita al menos dos centros, distintos, con monto positivo, y solo
//!   centros posibles del cargo.

use rust_decimal::Decimal;
use uuid::Uuid;

/// Mínimo de centros de costo para que un cargo se considere compartido.
pub const MIN_CENTROS: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MontoCentro {
    pub centro_costo_id: Uuid,
    pub monto: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorDistribucion {
    /// El cargo tiene menos de dos centros de costo posibles.
    NoCompartido,
    MenosDeDosCentros,
    CentroRepetido,
    MontoNoPositivo,
    CentroNoPosible,
}

impl ErrorDistribucion {
    pub fn mensaje(&self) -> &'static str {
        match self {
            Self::NoCompartido => {
                "Este apartado es solo para cargos que no pertenecen a un solo servicio."
            }
            Self::MenosDeDosCentros => "Indicá al menos dos centros de costo con su monto.",
            Self::CentroRepetido => "Cada centro de costo puede aparecer una sola vez.",
            Self::MontoNoPositivo => "Cada monto debe ser mayor a cero.",
            Self::CentroNoPosible => {
                "Solo se admiten centros de costo de los servicios donde está el cargo."
            }
        }
    }
}

/// Valida la distribución contra los centros posibles del cargo.
pub fn validar(posibles: &[Uuid], pares: &[MontoCentro]) -> Result<(), ErrorDistribucion> {
    if posibles.len() < MIN_CENTROS {
        return Err(ErrorDistribucion::NoCompartido);
    }
    if pares.len() < MIN_CENTROS {
        return Err(ErrorDistribucion::MenosDeDosCentros);
    }
    for (i, p) in pares.iter().enumerate() {
        if pares[..i]
            .iter()
            .any(|q| q.centro_costo_id == p.centro_costo_id)
        {
            return Err(ErrorDistribucion::CentroRepetido);
        }
        if p.monto <= Decimal::ZERO {
            return Err(ErrorDistribucion::MontoNoPositivo);
        }
        if !posibles.contains(&p.centro_costo_id) {
            return Err(ErrorDistribucion::CentroNoPosible);
        }
    }
    Ok(())
}

/// Sueldo total = suma de los montos.
pub fn total(pares: &[MontoCentro]) -> Decimal {
    pares.iter().map(|p| p.monto).sum()
}

/// % de cada par sobre el total, redondeado a 2 decimales (mismo orden que `pares`).
pub fn porcentajes(pares: &[MontoCentro]) -> Vec<Decimal> {
    let t = total(pares);
    pares
        .iter()
        .map(|p| {
            if t.is_zero() {
                Decimal::ZERO
            } else {
                (p.monto * Decimal::ONE_HUNDRED / t).round_dp(2)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn par(id: Uuid, monto: Decimal) -> MontoCentro {
        MontoCentro {
            centro_costo_id: id,
            monto,
        }
    }

    #[test]
    fn exige_cargo_compartido_y_dos_centros_validos() {
        let (a, b, c) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        assert_eq!(
            validar(&[a], &[par(a, dec!(1)), par(b, dec!(1))]),
            Err(ErrorDistribucion::NoCompartido)
        );
        let pos = [a, b];
        assert_eq!(
            validar(&pos, &[par(a, dec!(1))]),
            Err(ErrorDistribucion::MenosDeDosCentros)
        );
        assert_eq!(
            validar(&pos, &[par(a, dec!(1)), par(a, dec!(2))]),
            Err(ErrorDistribucion::CentroRepetido)
        );
        assert_eq!(
            validar(&pos, &[par(a, dec!(1)), par(b, dec!(0))]),
            Err(ErrorDistribucion::MontoNoPositivo)
        );
        assert_eq!(
            validar(&pos, &[par(a, dec!(1)), par(c, dec!(1))]),
            Err(ErrorDistribucion::CentroNoPosible)
        );
        assert_eq!(validar(&pos, &[par(a, dec!(1)), par(b, dec!(1))]), Ok(()));
    }

    #[test]
    fn porcentaje_sobre_el_total() {
        let (a, b, c) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let pares = [par(a, dec!(3000)), par(b, dec!(1000)), par(c, dec!(2000))];
        assert_eq!(total(&pares), dec!(6000));
        assert_eq!(
            porcentajes(&pares),
            vec![dec!(50), dec!(16.67), dec!(33.33)]
        );
    }
}

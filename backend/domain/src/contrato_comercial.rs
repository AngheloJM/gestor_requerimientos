use chrono::NaiveDate;

use crate::{ClienteId, ContratoComercialId, DomainError};

/// Cómo termina un contrato: `fecha_fin` y duración son alternativas, nunca ambas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinContrato {
    FechaFin(NaiveDate),
    /// Valores tal cual vienen de la base (sin interpretar).
    /// TODO(negocio): semántica de `duracion_unidad` (valores posibles) para calcular el fin.
    Duracion {
        valor: Option<String>,
        unidad: Option<String>,
    },
    /// Ni `fecha_fin` ni duración.
    /// TODO(negocio): ¿significa contrato indefinido o dato faltante?
    NoDefinido,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vigencia {
    Vigente,
    NoVigente,
    /// No se puede determinar con las reglas conocidas (duración o fin no definido).
    Indeterminada,
}

/// `contratos_comerciales(id, codigo unique, nombre, cliente_id nullable, fecha_inicio,
/// fecha_fin nullable, duracion_valor nullable, duracion_unidad nullable)`.
/// Se referencia desde `servicios.contrato_comercial_id` y tiene su propio `cliente_id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContratoComercial {
    pub id: ContratoComercialId,
    pub codigo: String,
    pub nombre: String,
    pub cliente_id: Option<ClienteId>,
    pub fecha_inicio: NaiveDate,
    pub fin: FinContrato,
}

impl ContratoComercial {
    /// Un argumento por columna de `contratos_comerciales`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: ContratoComercialId,
        codigo: String,
        nombre: String,
        cliente_id: Option<ClienteId>,
        fecha_inicio: NaiveDate,
        fecha_fin: Option<NaiveDate>,
        duracion_valor: Option<String>,
        duracion_unidad: Option<String>,
    ) -> Result<Self, DomainError> {
        let hay_duracion = duracion_valor.is_some() || duracion_unidad.is_some();
        let fin = match (fecha_fin, hay_duracion) {
            (Some(_), true) => return Err(DomainError::ContratoFinYDuracion),
            (Some(f), false) if f < fecha_inicio => return Err(DomainError::RangoFechasInvalido),
            (Some(f), false) => FinContrato::FechaFin(f),
            (None, true) => FinContrato::Duracion {
                valor: duracion_valor,
                unidad: duracion_unidad,
            },
            (None, false) => FinContrato::NoDefinido,
        };
        Ok(Self {
            id,
            codigo,
            nombre,
            cliente_id,
            fecha_inicio,
            fin,
        })
    }

    /// Vigencia en `fecha` (extremos inclusive). Solo se determina con `fecha_fin`;
    /// con duración o sin fin definido queda `Indeterminada` hasta definir la regla.
    pub fn vigencia_en(&self, fecha: NaiveDate) -> Vigencia {
        if fecha < self.fecha_inicio {
            return Vigencia::NoVigente;
        }
        match &self.fin {
            FinContrato::FechaFin(f) if fecha <= *f => Vigencia::Vigente,
            FinContrato::FechaFin(_) => Vigencia::NoVigente,
            FinContrato::Duracion { .. } | FinContrato::NoDefinido => Vigencia::Indeterminada,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn c(
        fin: Option<NaiveDate>,
        dv: Option<&str>,
        du: Option<&str>,
    ) -> Result<ContratoComercial, DomainError> {
        ContratoComercial::new(
            ContratoComercialId::new(),
            "CC-1".into(),
            "Contrato".into(),
            None,
            d(2026, 1, 1),
            fin,
            dv.map(Into::into),
            du.map(Into::into),
        )
    }

    #[test]
    fn fin_y_duracion_son_alternativas() {
        assert_eq!(
            c(Some(d(2026, 12, 31)), Some("12"), Some("x")),
            Err(DomainError::ContratoFinYDuracion)
        );
        assert_eq!(
            c(Some(d(2025, 1, 1)), None, None),
            Err(DomainError::RangoFechasInvalido)
        );
    }

    #[test]
    fn vigencia_con_fecha_fin_inclusiva() {
        let k = c(Some(d(2026, 12, 31)), None, None).unwrap();
        assert_eq!(k.vigencia_en(d(2025, 12, 31)), Vigencia::NoVigente);
        assert_eq!(k.vigencia_en(d(2026, 1, 1)), Vigencia::Vigente);
        assert_eq!(k.vigencia_en(d(2026, 12, 31)), Vigencia::Vigente);
        assert_eq!(k.vigencia_en(d(2027, 1, 1)), Vigencia::NoVigente);
    }

    #[test]
    fn duracion_o_sin_fin_es_indeterminada() {
        let k = c(None, Some("12"), Some("u")).unwrap();
        assert_eq!(k.vigencia_en(d(2026, 6, 1)), Vigencia::Indeterminada);
        // Antes del inicio sí se sabe: no vigente.
        assert_eq!(k.vigencia_en(d(2025, 6, 1)), Vigencia::NoVigente);
        let k = c(None, None, None).unwrap();
        assert_eq!(k.fin, FinContrato::NoDefinido);
        assert_eq!(k.vigencia_en(d(2026, 6, 1)), Vigencia::Indeterminada);
    }
}

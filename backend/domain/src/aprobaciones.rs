//! Aprobaciones: requerimientos con `fecha_fin` editable por RR.HH. y su historial completo
//! (`requerimiento_historial`). La autorización (quién ve / quién edita) la resuelve RLS.

use chrono::{DateTime, NaiveDate, Utc};

use crate::{CargoId, DomainError, RequerimientoId, ServicioId, UsuarioId};

/// Última edición de `fecha_fin` según `requerimiento_historial`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Modificacion {
    pub usuario_id: Option<UsuarioId>,
    pub fecha: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aprobacion {
    pub id: RequerimientoId,
    pub numero: i64,
    pub folio: Option<String>,
    pub tipo: Option<String>,
    pub estado: Option<String>,
    pub servicio_id: Option<ServicioId>,
    pub servicio_nombre: Option<String>,
    pub cargo_id: CargoId,
    pub cargo_nombre: Option<String>,
    /// `requerimientos.cantidad_personas` (nullable).
    pub cantidad: Option<u32>,
    pub fecha_inicio: NaiveDate,
    pub fecha_fin_tentativa: Option<NaiveDate>,
    /// Editable por RR.HH. (`requerimientos.fecha_fin`).
    pub fecha_fin: Option<NaiveDate>,
    pub fecha_inicio_operacion: Option<NaiveDate>,
    pub fecha_inicio_capacitacion: Option<NaiveDate>,
    pub motivo: Option<String>,
    /// Nombre del solicitante (si RLS permite ver su fila en `usuarios`).
    pub solicitante_nombre: Option<String>,
    pub ultima_modificacion: Option<Modificacion>,
    /// `requerimientos.updated_at`: versión para el control de concurrencia de
    /// `editar_requerimiento_con_historial` (el cliente la devuelve al editar).
    pub version: DateTime<Utc>,
}

/// Fila de `requerimiento_historial`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntradaHistorial {
    /// Como texto: el tipo de la PK de esta tabla no está confirmado.
    pub id: String,
    pub campo: String,
    pub valor_anterior: Option<String>,
    pub valor_nuevo: Option<String>,
    pub usuario_id: Option<UsuarioId>,
    pub created_at: DateTime<Utc>,
    pub glosa: Option<String>,
    pub grupo_edicion: Option<String>,
    pub categoria_desvio: Option<String>,
    pub detalle_desvio: Option<String>,
}

/// Regla: la fecha de fin no puede ser anterior a la de inicio (pueden coincidir).
pub fn validar_fecha_fin(
    fecha_inicio: NaiveDate,
    fecha_fin: Option<NaiveDate>,
) -> Result<(), DomainError> {
    match fecha_fin {
        Some(fin) if fin < fecha_inicio => Err(DomainError::RangoFechasInvalido),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, day).unwrap()
    }

    #[test]
    fn fecha_fin_valida() {
        assert_eq!(validar_fecha_fin(d(10), Some(d(10))), Ok(()));
        assert_eq!(validar_fecha_fin(d(10), None), Ok(()));
        assert_eq!(
            validar_fecha_fin(d(10), Some(d(9))),
            Err(DomainError::RangoFechasInvalido)
        );
    }
}

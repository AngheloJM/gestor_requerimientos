//! Requerimientos de personal (`requerimientos`): resumen para "Mis requerimientos".

use chrono::{DateTime, NaiveDate, Utc};

use crate::{CargoId, RequerimientoId, ServicioId, UsuarioId};

/// Valores admitidos por la base para `requerimientos.estado` (trigger `p2_validar_campos`,
/// verificado en producción). Se exponen como texto: filas antiguas podrían tener otros.
pub const ESTADOS: [&str; 5] = [
    "pendiente",
    "en_proceso",
    "aprobado",
    "rechazado",
    "cerrado",
];

/// Valores admitidos por la base para `requerimientos.motivo` (mismo trigger).
pub const MOTIVOS: [&str; 5] = [
    "Renuncia",
    "Baja médica",
    "Nuevo proyecto",
    "Reposición",
    "Crecimiento",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequerimientoResumen {
    pub id: RequerimientoId,
    pub numero: i64,
    pub folio: Option<String>,
    pub solicitante_id: Option<UsuarioId>,
    pub servicio_id: Option<ServicioId>,
    pub servicio_nombre: Option<String>,
    pub cargo_id: CargoId,
    pub cargo_nombre: Option<String>,
    pub tipo: Option<String>,
    pub motivo: Option<String>,
    pub cantidad: Option<u32>,
    pub estado: Option<String>,
    pub fecha_inicio: NaiveDate,
    pub fecha_inicio_operacion: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
}

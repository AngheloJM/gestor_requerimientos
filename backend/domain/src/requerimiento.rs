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

/// Valores admitidos por la base para `requerimientos.tipo` (CHECK `requerimientos_tipo_check`).
pub const TIPOS: [&str; 4] = ["nuevo", "modificacion", "promocion", "cambio"];

/// Valores admitidos por la base para `cargos.tipo` (CHECK `cargos_tipo_check`).
pub const TIPO_CARGO_OPERATIVO: &str = "operativo";
pub const TIPO_CARGO_STAFF: &str = "administrativo_staff";

/// Bono adicional a los configurados para el servicio+cargo (lo congela la base en `bonos_snap`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BonoExtra {
    pub nombre: String,
    pub monto: rust_decimal::Decimal,
    pub motivo: String,
}

/// Datos para crear un requerimiento. Las validaciones y el cálculo de la fecha de
/// capacitación los hace la función de la base `crear_requerimiento_con_snapshot`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NuevoRequerimiento {
    pub tipo: String,
    /// `None` solo para cargos `administrativo_staff`.
    pub servicio_id: Option<ServicioId>,
    pub cargo_id: CargoId,
    pub cantidad: u32,
    pub motivo: String,
    /// La base la recalcula (inicio de operación − días del servicio) para cargos operativos
    /// que no son promoción, si el servicio tiene días de capacitación.
    pub fecha_inicio_capacitacion: NaiveDate,
    pub fecha_inicio_operacion: NaiveDate,
    pub fecha_fin: Option<NaiveDate>,
    pub comentarios: Option<String>,
    pub bonos_extra: Vec<BonoExtra>,
}

/// Cargo elegible en el formulario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CargoOpcion {
    pub id: CargoId,
    pub nombre: String,
    /// `operativo` | `administrativo_staff` | sin definir.
    pub tipo: Option<String>,
}

/// Servicio activo donde el usuario puede crear requerimientos, con sus cargos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServicioOpcion {
    pub id: ServicioId,
    pub nombre: String,
    pub tiempo_capacitacion_dias: Option<i32>,
    pub cargos: Vec<CargoOpcion>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OpcionesRequerimiento {
    pub servicios: Vec<ServicioOpcion>,
    /// Cargos `administrativo_staff`: se pueden pedir sin servicio.
    pub cargos_staff: Vec<CargoOpcion>,
}

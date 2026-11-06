//! Tablero: dos piezas **independientes**.
//!
//! 1. **Indicadores (3 cuadros de arriba)** derivados de la completitud de cada servicio activo.
//! 2. **Kanban (abajo)** = `solicitudes_cambio`, enrutado a un área según el valor de `campo`.

use chrono::{DateTime, NaiveDate, Utc};

use crate::{
    CargoId, ContratoComercial, ServicioId, SolicitudCambioId, UsuarioId,
    contrato_comercial::Vigencia,
};

// ---------------------------------------------------------------------------------------
// 1. Completitud → indicadores
// ---------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Area {
    Comercial,
    Operaciones,
    Rrhh,
}

impl Area {
    pub const TODAS: [Area; 3] = [Area::Comercial, Area::Operaciones, Area::Rrhh];

    pub fn as_str(self) -> &'static str {
        match self {
            Area::Comercial => "comercial",
            Area::Operaciones => "operaciones",
            Area::Rrhh => "rrhh",
        }
    }
}

/// Estado del contrato comercial de un servicio (`servicios.contrato_comercial_id`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EstadoContrato {
    SinContrato,
    /// El servicio referencia un contrato que no es visible para el usuario (RLS).
    NoVisible,
    Con(ContratoComercial),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EstadoServicio {
    pub servicio_id: ServicioId,
    pub servicio_nombre: String,
    pub cliente_nombre: Option<String>,
    pub contrato: EstadoContrato,
    /// Filas en `servicio_cargos` del servicio.
    pub cantidad_cargos: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TareaCompletitud {
    AsignarContratoComercial,
    ContratoNoVigente,
    /// Contrato con duración o sin fin definido: no se puede afirmar que esté vigente.
    VerificarVigenciaContrato,
    DefinirCargos,
}

impl TareaCompletitud {
    /// SUPUESTO (ver docs/PENDIENTES.md): contrato → comercial; cargos → operaciones.
    pub fn area(self) -> Area {
        match self {
            Self::AsignarContratoComercial
            | Self::ContratoNoVigente
            | Self::VerificarVigenciaContrato => Area::Comercial,
            Self::DefinirCargos => Area::Operaciones,
        }
    }

    pub fn codigo(self) -> &'static str {
        match self {
            Self::AsignarContratoComercial => "asignar_contrato_comercial",
            Self::ContratoNoVigente => "contrato_no_vigente",
            Self::VerificarVigenciaContrato => "verificar_vigencia_contrato",
            Self::DefinirCargos => "definir_cargos",
        }
    }
}

/// Criterios de completitud que todavía no se evalúan por falta de definición.
/// TODO(negocio): criterio de "composición salarial completa". Con el schema real cada fila
/// de `servicio_cargos` ya exige `haber_basico_diurno`; no está definido qué la haría
/// "incompleta" (¿nocturno según turno? ¿métricas configuradas?).
pub const CRITERIOS_SIN_DEFINIR: &[&str] = &["composicion_salarial_completa"];

impl EstadoServicio {
    pub fn pendientes(&self, hoy: NaiveDate) -> Vec<TareaCompletitud> {
        let mut v = Vec::new();
        match &self.contrato {
            EstadoContrato::SinContrato => v.push(TareaCompletitud::AsignarContratoComercial),
            EstadoContrato::NoVisible => v.push(TareaCompletitud::VerificarVigenciaContrato),
            EstadoContrato::Con(c) => match c.vigencia_en(hoy) {
                Vigencia::Vigente => {}
                Vigencia::NoVigente => v.push(TareaCompletitud::ContratoNoVigente),
                Vigencia::Indeterminada => v.push(TareaCompletitud::VerificarVigenciaContrato),
            },
        }
        if self.cantidad_cargos == 0 {
            v.push(TareaCompletitud::DefinirCargos);
        }
        v
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Indicadores {
    pub servicios_total: u32,
    pub servicios_configurados: u32,
    pub servicios_incompletos: u32,
    /// Áreas (0–3) con al menos un pendiente de completitud.
    pub areas_sin_completar: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServicioIncompleto {
    pub servicio_id: ServicioId,
    pub servicio_nombre: String,
    pub cliente_nombre: Option<String>,
    pub tareas: Vec<TareaCompletitud>,
}

/// Criterios que hoy se evalúan por servicio: (1) contrato comercial vigente y (2) cargos
/// definidos. Las tareas de contrato son excluyentes entre sí, así que cada tarea pendiente
/// equivale a un criterio incumplido.
/// SUPUESTO (a confirmar): todos los criterios pesan igual en el % de completitud.
pub const CRITERIOS_EVALUADOS: u32 = 2;

impl ServicioIncompleto {
    /// % de criterios cumplidos (0–100), redondeado hacia abajo.
    pub fn porcentaje_completitud(&self) -> u32 {
        let pendientes = u32::try_from(self.tareas.len()).unwrap_or(u32::MAX);
        CRITERIOS_EVALUADOS.saturating_sub(pendientes) * 100 / CRITERIOS_EVALUADOS
    }
}

/// Servicio activo sin tareas pendientes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServicioConfigurado {
    pub servicio_id: ServicioId,
    pub servicio_nombre: String,
    pub cliente_nombre: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completitud {
    pub indicadores: Indicadores,
    pub configurados: Vec<ServicioConfigurado>,
    pub incompletos: Vec<ServicioIncompleto>,
}

impl Completitud {
    pub fn construir(servicios: &[EstadoServicio], hoy: NaiveDate) -> Self {
        let mut incompletos = Vec::new();
        let mut configurados = Vec::new();
        let mut areas = std::collections::BTreeSet::new();
        for s in servicios {
            let tareas = s.pendientes(hoy);
            if tareas.is_empty() {
                configurados.push(ServicioConfigurado {
                    servicio_id: s.servicio_id,
                    servicio_nombre: s.servicio_nombre.clone(),
                    cliente_nombre: s.cliente_nombre.clone(),
                });
            } else {
                areas.extend(tareas.iter().map(|t| t.area()));
                incompletos.push(ServicioIncompleto {
                    servicio_id: s.servicio_id,
                    servicio_nombre: s.servicio_nombre.clone(),
                    cliente_nombre: s.cliente_nombre.clone(),
                    tareas,
                });
            }
        }
        let total = u32::try_from(servicios.len()).unwrap_or(u32::MAX);
        let n_incompletos = u32::try_from(incompletos.len()).unwrap_or(u32::MAX);
        Self {
            indicadores: Indicadores {
                servicios_total: total,
                servicios_configurados: total - n_incompletos,
                servicios_incompletos: n_incompletos,
                areas_sin_completar: u32::try_from(areas.len()).unwrap_or(3),
            },
            configurados,
            incompletos,
        }
    }
}

// ---------------------------------------------------------------------------------------
// 2. Kanban ← solicitudes_cambio
// ---------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolicitudCambio {
    pub id: SolicitudCambioId,
    pub servicio_id: ServicioId,
    pub servicio_nombre: Option<String>,
    pub cargo_id: Option<CargoId>,
    pub cargo_nombre: Option<String>,
    pub campo: String,
    pub solicitante_id: Option<UsuarioId>,
    pub motivo: Option<String>,
    pub descripcion: Option<String>,
    pub estado: Option<String>,
    pub responsable_id: Option<UsuarioId>,
    pub comentario_resolucion: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub cerrada_at: Option<DateTime<Utc>>,
}

/// Columna del Kanban según `campo`.
///
/// Regla conocida: por defecto una solicitud la ve solo RR.HH. (y el solicitante); con
/// `campo = 'centro_costo'` también la ve Costos. **Quién ve cada tarjeta lo resuelve RLS**,
/// no este código.
///
/// TODO(negocio): qué valores de `campo` se enrutan a las columnas Comercial y Operaciones.
/// Hasta definirlo, todas las solicitudes van a la columna RR.HH.
pub fn area_kanban(_campo: &str) -> Area {
    Area::Rrhh
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Kanban {
    pub comercial: Vec<SolicitudCambio>,
    pub operaciones: Vec<SolicitudCambio>,
    pub rrhh: Vec<SolicitudCambio>,
}

impl Kanban {
    pub fn construir(solicitudes: Vec<SolicitudCambio>) -> Self {
        let mut k = Kanban::default();
        for s in solicitudes {
            match area_kanban(&s.campo) {
                Area::Comercial => k.comercial.push(s),
                Area::Operaciones => k.operaciones.push(s),
                Area::Rrhh => k.rrhh.push(s),
            }
        }
        k
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ContratoComercialId;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn contrato(fin: Option<NaiveDate>, duracion: bool) -> EstadoContrato {
        EstadoContrato::Con(
            ContratoComercial::new(
                ContratoComercialId::new(),
                "C".into(),
                "C".into(),
                None,
                d(2026, 1, 1),
                fin,
                duracion.then(|| "12".into()),
                duracion.then(|| "u".into()),
            )
            .unwrap(),
        )
    }

    fn s(nombre: &str, contrato: EstadoContrato, cargos: u32) -> EstadoServicio {
        EstadoServicio {
            servicio_id: ServicioId::new(),
            servicio_nombre: nombre.into(),
            cliente_nombre: None,
            contrato,
            cantidad_cargos: cargos,
        }
    }

    #[test]
    fn completitud_por_estado_de_contrato_y_cargos() {
        let hoy = d(2026, 11, 2);
        let c = Completitud::construir(
            &[
                s("ok", contrato(Some(d(2026, 12, 31)), false), 2),
                s("sin contrato", EstadoContrato::SinContrato, 1),
                s("vencido", contrato(Some(d(2026, 6, 30)), false), 1),
                s("por duración", contrato(None, true), 1),
                s("sin cargos", contrato(Some(d(2026, 12, 31)), false), 0),
            ],
            hoy,
        );
        assert_eq!(
            c.indicadores,
            Indicadores {
                servicios_total: 5,
                servicios_configurados: 1,
                servicios_incompletos: 4,
                areas_sin_completar: 2,
            }
        );
        assert_eq!(c.configurados.len(), 1);
        assert_eq!(c.configurados[0].servicio_nombre, "ok");
        let pct: Vec<_> = c.incompletos.iter().map(|i| i.porcentaje_completitud()).collect();
        assert_eq!(pct, vec![50, 50, 50, 50]);
        let codigos: Vec<_> = c
            .incompletos
            .iter()
            .map(|i| (i.servicio_nombre.as_str(), i.tareas[0].codigo()))
            .collect();
        assert_eq!(
            codigos,
            [
                ("sin contrato", "asignar_contrato_comercial"),
                ("vencido", "contrato_no_vigente"),
                ("por duración", "verificar_vigencia_contrato"),
                ("sin cargos", "definir_cargos"),
            ]
        );
    }

    #[test]
    fn kanban_por_defecto_va_a_rrhh() {
        let sol = |campo: &str| SolicitudCambio {
            id: SolicitudCambioId::new(),
            servicio_id: ServicioId::new(),
            servicio_nombre: None,
            cargo_id: None,
            cargo_nombre: None,
            campo: campo.into(),
            solicitante_id: None,
            motivo: None,
            descripcion: None,
            estado: None,
            responsable_id: None,
            comentario_resolucion: None,
            created_at: DateTime::UNIX_EPOCH,
            updated_at: None,
            cerrada_at: None,
        };
        let k = Kanban::construir(vec![sol("centro_costo"), sol("haber_basico_diurno")]);
        assert_eq!(k.rrhh.len(), 2);
        assert!(k.comercial.is_empty() && k.operaciones.is_empty());
    }
}

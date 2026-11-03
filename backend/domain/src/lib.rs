//! Dominio: entidades y reglas de negocio, sin dependencias de HTTP ni de base de datos.
//!
//! La **autorización no vive aquí**: la resuelven las políticas RLS de Postgres (ver
//! `db::rls`). Este crate solo modela datos y reglas de cálculo.

pub mod almanaque;
pub mod aprobaciones;
pub mod error;
pub mod ids;

pub mod cargo;
pub mod centro_costo;
pub mod cliente;
pub mod composicion_salarial;
pub mod contrato_comercial;
pub mod matriz;
pub mod proyecto;
pub mod rol;
pub mod servicio;
pub mod tablero;

pub use almanaque::{
    Almanaque, DiaAlmanaque, LineaAlmanaque, Movimiento, Periodo, PeriodoInvalido, TipoMovimiento,
};
pub use aprobaciones::{Aprobacion, EntradaHistorial, Modificacion};
pub use cargo::Cargo;
pub use centro_costo::CentroCosto;
pub use cliente::Cliente;
pub use composicion_salarial::{ComposicionSalarial, MetricaBono, Umbral};
pub use contrato_comercial::{ContratoComercial, FinContrato, Vigencia};
pub use error::DomainError;
pub use ids::*;
pub use matriz::{FilaMatriz, FiltrosMatriz};
pub use proyecto::Proyecto;
pub use rol::{Rol, RolDesconocido};
pub use servicio::Servicio;
pub use tablero::{
    Area, Completitud, EstadoContrato, EstadoServicio, Indicadores, Kanban, ServicioIncompleto,
    SolicitudCambio, TareaCompletitud,
};

/// Montos de dinero: decimal exacto (nunca f64).
pub use rust_decimal::Decimal;

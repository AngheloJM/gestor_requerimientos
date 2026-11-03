//! Matriz de servicio: Composición Salarial concatenada en el orden de negocio obligatorio
//! **Centro de Costo → Cliente → Servicio → Cargo** (decisión del cliente final; no cambiar).
//! Proyecto se incluye como dato adicional/filtro, nunca antes de Centro de Costo.

use crate::{
    Cargo, CentroCosto, CentroCostoId, Cliente, ClienteId, ComposicionSalarial, Proyecto,
    ProyectoId, Servicio, ServicioId,
};

/// Filtros opcionales; `None` = sin filtrar. Se combinan con AND.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FiltrosMatriz {
    pub servicio_id: Option<ServicioId>,
    pub cliente_id: Option<ClienteId>,
    pub centro_costo_id: Option<CentroCostoId>,
    pub proyecto_id: Option<ProyectoId>,
}

/// Una fila: una composición salarial (servicio+cargo) con su contexto.
/// Centro de costo, cliente y proyecto son opcionales porque las FKs de `servicios` lo son.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilaMatriz {
    pub centro_costo: Option<CentroCosto>,
    pub cliente: Option<Cliente>,
    pub proyecto: Option<Proyecto>,
    pub servicio: Servicio,
    pub cargo: Cargo,
    pub composicion: ComposicionSalarial,
}

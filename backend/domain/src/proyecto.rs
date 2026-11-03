use crate::{ClienteId, ProyectoId};

/// `proyectos(id, cliente_id NOT NULL, nombre, created_at, updated_at)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proyecto {
    pub id: ProyectoId,
    pub cliente_id: ClienteId,
    pub nombre: String,
}

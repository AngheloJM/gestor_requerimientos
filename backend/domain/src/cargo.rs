use crate::CargoId;

/// `cargos(id, nombre, tipo, created_at)`: **catálogo compartido**, no pertenece a un
/// servicio. La relación servicio↔cargo es `servicio_cargos` (ver `ComposicionSalarial`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cargo {
    pub id: CargoId,
    pub nombre: String,
    pub tipo: Option<String>,
}

use crate::CentroCostoId;

/// `centros_costo(id, codigo unique, nombre, created_at)`.
/// Independiente de `Cliente` (no tiene `cliente_id`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CentroCosto {
    pub id: CentroCostoId,
    pub codigo: String,
    pub nombre: String,
}

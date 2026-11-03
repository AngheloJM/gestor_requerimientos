use crate::ClienteId;

/// `clientes(id, nombre unique, logo_url, manual_marca_url, created_at)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cliente {
    pub id: ClienteId,
    pub nombre: String,
    pub logo_url: Option<String>,
    pub manual_marca_url: Option<String>,
}

//! DTOs compartidos.

use rust_decimal::Decimal;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct RefDto {
    pub id: Uuid,
    pub nombre: Option<String>,
}

impl RefDto {
    pub fn new(id: impl Into<Uuid>, nombre: Option<String>) -> Self {
        Self {
            id: id.into(),
            nombre,
        }
    }
}

/// Montos de dinero: siempre 2 decimales en la respuesta ("0.00", "2500.00"),
/// serializados como string para no perder precisión.
pub fn monto(mut d: Decimal) -> Decimal {
    d.rescale(2);
    d
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn montos_con_dos_decimales() {
        assert_eq!(monto(Decimal::ZERO).to_string(), "0.00");
        assert_eq!(monto(dec!(2500)).to_string(), "2500.00");
    }
}

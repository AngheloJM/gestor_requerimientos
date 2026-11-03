use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    /// La fecha de fin es anterior a la de inicio.
    RangoFechasInvalido,
    /// `contratos_comerciales` con `fecha_fin` y duración a la vez (son alternativas).
    ContratoFinYDuracion,
    /// Una cantidad que debe ser >= 0 es negativa o excede el rango.
    CantidadInvalida { campo: &'static str, valor: i64 },
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RangoFechasInvalido => write!(f, "la fecha de fin es anterior a la de inicio"),
            Self::ContratoFinYDuracion => write!(
                f,
                "el contrato tiene fecha_fin y duración a la vez (son alternativas)"
            ),
            Self::CantidadInvalida { campo, valor } => {
                write!(f, "la cantidad `{campo}` es inválida ({valor})")
            }
        }
    }
}

impl std::error::Error for DomainError {}

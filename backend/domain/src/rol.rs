use std::{fmt, str::FromStr};

/// Rol de aplicación de un usuario (columna `usuarios.rol`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rol {
    RrhhAdmin,
    Estandar,
    EncargadoCostos,
    Externo,
}

impl Rol {
    pub const TODOS: [Rol; 4] = [
        Rol::RrhhAdmin,
        Rol::Estandar,
        Rol::EncargadoCostos,
        Rol::Externo,
    ];

    /// Representación textual, idéntica a la de la base de datos.
    pub fn as_str(self) -> &'static str {
        match self {
            Rol::RrhhAdmin => "rrhh_admin",
            Rol::Estandar => "estandar",
            Rol::EncargadoCostos => "encargado_costos",
            Rol::Externo => "externo",
        }
    }
}

impl fmt::Display for Rol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RolDesconocido(pub String);

impl FromStr for Rol {
    type Err = RolDesconocido;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Rol::TODOS
            .into_iter()
            .find(|r| r.as_str() == s)
            .ok_or_else(|| RolDesconocido(s.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ida_y_vuelta_texto() {
        for rol in Rol::TODOS {
            assert_eq!(rol.as_str().parse::<Rol>(), Ok(rol));
        }
        assert!("admin".parse::<Rol>().is_err());
        assert!("RRHH_ADMIN".parse::<Rol>().is_err());
    }
}

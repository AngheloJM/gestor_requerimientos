//! Roles y permisos configurables (fases 0 y 1 de docs/PLAN_ROLES.md).
//!
//! En esta fase los permisos se registran pero NO se aplican: el acceso real lo deciden las
//! políticas RLS actuales. La autorización para administrar roles también la aplica RLS.

use chrono::{DateTime, Utc};

use crate::UsuarioId;

/// Alcances admitidos por la base (`rol_permisos.alcance`).
pub const ALCANCES: [&str; 3] = ["todos", "servicios_asignados", "propios"];

/// Entrada del catálogo `permisos`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Permiso {
    pub clave: String,
    pub modulo: String,
    pub recurso: String,
    pub recurso_nombre: String,
    pub accion: String,
    pub descripcion: String,
    pub admite_alcance: bool,
    pub orden: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermisoAsignado {
    pub clave: String,
    pub alcance: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rol {
    pub id: uuid::Uuid,
    pub clave: String,
    pub nombre: String,
    pub descripcion: Option<String>,
    pub es_sistema: bool,
    pub activo: bool,
    pub creado_por: Option<UsuarioId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub permisos: Vec<PermisoAsignado>,
}

/// Datos editables de un rol (crear o reemplazar).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatosRol {
    pub nombre: String,
    pub descripcion: Option<String>,
    pub activo: bool,
    pub permisos: Vec<PermisoAsignado>,
}

/// Clave técnica a partir del nombre: minúsculas ASCII, `_` como separador, empieza con letra.
/// "Gerente de Área" → "gerente_de_area". `None` si no queda nada utilizable.
pub fn clave_desde_nombre(nombre: &str) -> Option<String> {
    let mut clave = String::new();
    for c in nombre.trim().to_lowercase().chars() {
        let c = match c {
            'á' | 'à' | 'ä' | 'â' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            'ñ' => 'n',
            c => c,
        };
        if c.is_ascii_alphanumeric() {
            clave.push(c);
        } else if !clave.is_empty() && !clave.ends_with('_') {
            clave.push('_');
        }
    }
    let clave = clave.trim_end_matches('_');
    let clave = clave.trim_start_matches(|c: char| c.is_ascii_digit() || c == '_');
    let clave: String = clave.chars().take(50).collect();
    let clave = clave.trim_end_matches('_').to_owned();
    (clave.len() >= 2).then_some(clave)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clave_normaliza_acentos_espacios_y_simbolos() {
        assert_eq!(
            clave_desde_nombre("Gerente de Área").as_deref(),
            Some("gerente_de_area")
        );
        assert_eq!(
            clave_desde_nombre("  Jefe   (Operaciones) ").as_deref(),
            Some("jefe_operaciones")
        );
        assert_eq!(clave_desde_nombre("Ñandú 2").as_deref(), Some("nandu_2"));
        assert_eq!(clave_desde_nombre("3er nivel").as_deref(), Some("er_nivel"));
        assert_eq!(clave_desde_nombre("!!!"), None);
        assert_eq!(clave_desde_nombre("a"), None);
        assert!(clave_desde_nombre(&"x".repeat(80)).unwrap().len() <= 50);
    }
}

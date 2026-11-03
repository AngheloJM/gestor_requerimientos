//! Perfil del usuario autenticado (`usuarios`, `id` = `auth.users.id`).

use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::DbError;

#[derive(Debug, Clone, FromRow)]
pub struct Perfil {
    pub id: Uuid,
    pub nombre: Option<String>,
    pub rol: Option<String>,
    pub activo: Option<bool>,
}

/// Perfil del usuario, si RLS permite verlo.
pub async fn perfil(conn: &mut PgConnection, id: Uuid) -> Result<Option<Perfil>, DbError> {
    Ok(sqlx::query_as(
        "SELECT id, nombre::text AS nombre, rol::text AS rol, activo FROM usuarios WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(conn)
    .await?)
}

// ---------------------------------------------------------------------------------------
// Administración de usuarios (pantalla "Usuarios", solo RR.HH.: lo aplica RLS)
// ---------------------------------------------------------------------------------------

/// Fila de la lista de usuarios con sus servicios asignados.
#[derive(Debug, Clone)]
pub struct UsuarioAdmin {
    pub id: Uuid,
    pub nombre: String,
    pub cargo_id: Option<Uuid>,
    pub cargo_nombre: Option<String>,
    pub division: Option<String>,
    pub region: Option<String>,
    pub inmediato_superior_id: Option<Uuid>,
    pub superior_nombre: Option<String>,
    pub rol: String,
    pub activo: bool,
    /// (id, nombre) de `usuario_servicios`.
    pub servicios: Vec<(Uuid, String)>,
}

#[derive(FromRow)]
struct UsuarioRow {
    id: Uuid,
    nombre: String,
    cargo_id: Option<Uuid>,
    cargo_nombre: Option<String>,
    division: Option<String>,
    region: Option<String>,
    inmediato_superior_id: Option<Uuid>,
    superior_nombre: Option<String>,
    rol: String,
    activo: bool,
}

#[derive(FromRow)]
struct AsignacionRow {
    usuario_id: Uuid,
    servicio_id: Uuid,
    servicio_nombre: String,
}

const QUERY_USUARIOS: &str = r#"
SELECT u.id, u.nombre::text AS nombre, u.cargo_id, c.nombre::text AS cargo_nombre,
       u.division::text AS division, u.region::text AS region,
       u.inmediato_superior_id, sup.nombre::text AS superior_nombre,
       u.rol::text AS rol, u.activo
FROM usuarios u
LEFT JOIN cargos   c   ON c.id = u.cargo_id
LEFT JOIN usuarios sup ON sup.id = u.inmediato_superior_id
ORDER BY lower(u.nombre), u.id
"#;

const QUERY_ASIGNACIONES: &str = r#"
SELECT us.usuario_id, us.servicio_id, s.nombre::text AS servicio_nombre
FROM usuario_servicios us JOIN servicios s ON s.id = us.servicio_id
ORDER BY lower(s.nombre)
"#;

pub async fn listar_admin(conn: &mut PgConnection) -> Result<Vec<UsuarioAdmin>, DbError> {
    let filas: Vec<UsuarioRow> = sqlx::query_as(QUERY_USUARIOS).fetch_all(&mut *conn).await?;
    let asignaciones: Vec<AsignacionRow> =
        sqlx::query_as(QUERY_ASIGNACIONES).fetch_all(conn).await?;
    let mut por_usuario: std::collections::HashMap<Uuid, Vec<(Uuid, String)>> =
        std::collections::HashMap::new();
    for a in asignaciones {
        por_usuario
            .entry(a.usuario_id)
            .or_default()
            .push((a.servicio_id, a.servicio_nombre));
    }
    Ok(filas
        .into_iter()
        .map(|u| UsuarioAdmin {
            servicios: por_usuario.remove(&u.id).unwrap_or_default(),
            id: u.id,
            nombre: u.nombre,
            cargo_id: u.cargo_id,
            cargo_nombre: u.cargo_nombre,
            division: u.division,
            region: u.region,
            inmediato_superior_id: u.inmediato_superior_id,
            superior_nombre: u.superior_nombre,
            rol: u.rol,
            activo: u.activo,
        })
        .collect())
}

/// Datos editables (los mismos que la pantalla original). `nombre` no se edita aquí.
#[derive(Debug, Clone)]
pub struct DatosUsuario {
    pub cargo_id: Option<Uuid>,
    pub division: Option<String>,
    pub region: Option<String>,
    pub inmediato_superior_id: Option<Uuid>,
    /// Uno de los valores del CHECK `usuarios_rol_check`.
    pub rol: String,
    pub activo: bool,
    pub servicios: Vec<Uuid>,
}

#[derive(Debug)]
pub enum EditarUsuarioError {
    /// No existe o RLS no permite modificarlo.
    NoEncontrado,
    Db(DbError),
}

impl From<sqlx::Error> for EditarUsuarioError {
    fn from(e: sqlx::Error) -> Self {
        Self::Db(DbError::Sqlx(e))
    }
}

/// Actualiza el usuario y sincroniza sus servicios asignados (solo las diferencias).
pub async fn actualizar_admin(
    conn: &mut PgConnection,
    id: Uuid,
    d: &DatosUsuario,
) -> Result<(), EditarUsuarioError> {
    let limpio = |v: &Option<String>| {
        v.as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    };
    let n = sqlx::query(
        "UPDATE usuarios SET cargo_id = $2, division = $3, region = $4,
                inmediato_superior_id = $5, rol = $6, activo = $7
         WHERE id = $1",
    )
    .bind(id)
    .bind(d.cargo_id)
    .bind(limpio(&d.division))
    .bind(limpio(&d.region))
    .bind(d.inmediato_superior_id)
    .bind(&d.rol)
    .bind(d.activo)
    .execute(&mut *conn)
    .await?
    .rows_affected();
    if n == 0 {
        return Err(EditarUsuarioError::NoEncontrado);
    }
    sqlx::query(
        "DELETE FROM usuario_servicios WHERE usuario_id = $1 AND servicio_id <> ALL($2::uuid[])",
    )
    .bind(id)
    .bind(&d.servicios)
    .execute(&mut *conn)
    .await?;
    sqlx::query(
        "INSERT INTO usuario_servicios (usuario_id, servicio_id)
         SELECT $1, s FROM unnest($2::uuid[]) AS s ON CONFLICT DO NOTHING",
    )
    .bind(id)
    .bind(&d.servicios)
    .execute(conn)
    .await?;
    Ok(())
}

/// Opciones del formulario: catálogo de cargos y servicios activos (para asignar).
pub async fn opciones_admin(
    conn: &mut PgConnection,
) -> Result<(Vec<(Uuid, String)>, Vec<(Uuid, String)>), DbError> {
    let cargos: Vec<(Uuid, String)> =
        sqlx::query_as("SELECT id, nombre::text FROM cargos ORDER BY lower(nombre)")
            .fetch_all(&mut *conn)
            .await?;
    let servicios: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id, nombre::text FROM servicios WHERE activo IS TRUE ORDER BY lower(nombre)",
    )
    .fetch_all(conn)
    .await?;
    Ok((cargos, servicios))
}

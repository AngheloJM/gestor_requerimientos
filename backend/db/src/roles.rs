//! Roles y permisos configurables (tablas de la migración 0003). Autorización: RLS
//! (leer: autenticados; administrar: rrhh_admin). Reglas (roles de sistema inmutables,
//! alcance válido, auditoría) en triggers de la base.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use domain::{
    DatosRol, Permiso, PermisoAsignado,
    roles::{Rol, clave_desde_nombre},
};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::DbError;

#[derive(Debug)]
pub enum RolError {
    NoEncontrado,
    /// Nombre vacío o sin caracteres utilizables.
    NombreInvalido,
    /// Ya existe un rol con ese nombre (o clave).
    Duplicado,
    Db(DbError),
}

impl From<sqlx::Error> for RolError {
    fn from(e: sqlx::Error) -> Self {
        let e = DbError::Sqlx(e);
        if e.es_duplicado() {
            Self::Duplicado
        } else {
            Self::Db(e)
        }
    }
}

#[derive(FromRow)]
struct PermisoRow {
    clave: String,
    modulo: String,
    recurso: String,
    recurso_nombre: String,
    accion: String,
    descripcion: String,
    admite_alcance: bool,
    orden: i32,
}

#[derive(FromRow)]
struct RolRow {
    id: Uuid,
    clave: String,
    nombre: String,
    descripcion: Option<String>,
    es_sistema: bool,
    activo: bool,
    creado_por: Option<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct AsignadoRow {
    rol_id: Uuid,
    permiso_clave: String,
    alcance: String,
}

pub async fn catalogo(conn: &mut PgConnection) -> Result<Vec<Permiso>, DbError> {
    let rows: Vec<PermisoRow> = sqlx::query_as(
        "SELECT clave, modulo, recurso, recurso_nombre, accion, descripcion, admite_alcance, orden
         FROM permisos ORDER BY orden, clave",
    )
    .fetch_all(conn)
    .await?;
    Ok(rows
        .into_iter()
        .map(|p| Permiso {
            clave: p.clave,
            modulo: p.modulo,
            recurso: p.recurso,
            recurso_nombre: p.recurso_nombre,
            accion: p.accion,
            descripcion: p.descripcion,
            admite_alcance: p.admite_alcance,
            orden: p.orden,
        })
        .collect())
}

const LISTAR_ROLES: &str = "SELECT id, clave, nombre, descripcion, es_sistema, activo, creado_por,
                                   created_at, updated_at
                            FROM roles ORDER BY es_sistema DESC, lower(nombre)";
const OBTENER_ROL: &str = "SELECT id, clave, nombre, descripcion, es_sistema, activo, creado_por,
                                  created_at, updated_at
                           FROM roles WHERE id = $1";

fn armar(r: RolRow, permisos: Vec<PermisoAsignado>) -> Rol {
    Rol {
        id: r.id,
        clave: r.clave,
        nombre: r.nombre,
        descripcion: r.descripcion,
        es_sistema: r.es_sistema,
        activo: r.activo,
        creado_por: r.creado_por.map(Into::into),
        created_at: r.created_at,
        updated_at: r.updated_at,
        permisos,
    }
}

/// Todos los roles (sistema primero, luego por nombre) con sus permisos.
pub async fn listar(conn: &mut PgConnection) -> Result<Vec<Rol>, DbError> {
    let roles: Vec<RolRow> = sqlx::query_as(LISTAR_ROLES).fetch_all(&mut *conn).await?;
    let asignados: Vec<AsignadoRow> = sqlx::query_as(
        "SELECT rp.rol_id, rp.permiso_clave, rp.alcance FROM rol_permisos rp
         JOIN permisos p ON p.clave = rp.permiso_clave ORDER BY p.orden",
    )
    .fetch_all(conn)
    .await?;
    let mut por_rol: HashMap<Uuid, Vec<PermisoAsignado>> = HashMap::new();
    for a in asignados {
        por_rol.entry(a.rol_id).or_default().push(PermisoAsignado {
            clave: a.permiso_clave,
            alcance: a.alcance,
        });
    }
    Ok(roles
        .into_iter()
        .map(|r| {
            let p = por_rol.remove(&r.id).unwrap_or_default();
            armar(r, p)
        })
        .collect())
}

pub async fn obtener(conn: &mut PgConnection, id: Uuid) -> Result<Option<Rol>, DbError> {
    let Some(r): Option<RolRow> = sqlx::query_as(OBTENER_ROL)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await?
    else {
        return Ok(None);
    };
    let asignados: Vec<AsignadoRow> = sqlx::query_as(
        "SELECT rp.rol_id, rp.permiso_clave, rp.alcance FROM rol_permisos rp
         JOIN permisos p ON p.clave = rp.permiso_clave WHERE rp.rol_id = $1 ORDER BY p.orden",
    )
    .bind(id)
    .fetch_all(conn)
    .await?;
    let permisos = asignados
        .into_iter()
        .map(|a| PermisoAsignado {
            clave: a.permiso_clave,
            alcance: a.alcance,
        })
        .collect();
    Ok(Some(armar(r, permisos)))
}

/// Reemplaza los permisos del rol aplicando solo las diferencias (así el historial registra
/// exactamente qué se otorgó, revocó o cambió de alcance).
async fn sincronizar_permisos(
    conn: &mut PgConnection,
    rol: Uuid,
    permisos: &[PermisoAsignado],
) -> Result<(), RolError> {
    let claves: Vec<&str> = permisos.iter().map(|p| p.clave.as_str()).collect();
    let alcances: Vec<&str> = permisos.iter().map(|p| p.alcance.as_str()).collect();
    sqlx::query("DELETE FROM rol_permisos WHERE rol_id = $1 AND permiso_clave <> ALL($2::text[])")
        .bind(rol)
        .bind(&claves)
        .execute(&mut *conn)
        .await?;
    sqlx::query(
        "INSERT INTO rol_permisos (rol_id, permiso_clave, alcance)
         SELECT $1, c, a FROM unnest($2::text[], $3::text[]) AS x(c, a)
         ON CONFLICT (rol_id, permiso_clave) DO UPDATE SET alcance = EXCLUDED.alcance
         WHERE rol_permisos.alcance IS DISTINCT FROM EXCLUDED.alcance",
    )
    .bind(rol)
    .bind(&claves)
    .bind(&alcances)
    .execute(conn)
    .await?;
    Ok(())
}

fn limpiar(d: &DatosRol) -> (String, Option<String>) {
    let nombre = d.nombre.split_whitespace().collect::<Vec<_>>().join(" ");
    let descripcion = d
        .descripcion
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    (nombre, descripcion)
}

/// Crea un rol con sus permisos. Devuelve su id.
pub async fn crear(conn: &mut PgConnection, d: &DatosRol) -> Result<Uuid, RolError> {
    let (nombre, descripcion) = limpiar(d);
    let clave = clave_desde_nombre(&nombre).ok_or(RolError::NombreInvalido)?;
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO roles (clave, nombre, descripcion, activo) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(&clave)
    .bind(&nombre)
    .bind(&descripcion)
    .bind(d.activo)
    .fetch_one(&mut *conn)
    .await?;
    sincronizar_permisos(conn, id, &d.permisos).await?;
    Ok(id)
}

/// Actualiza nombre, descripción, estado y permisos (la clave no cambia).
pub async fn actualizar(conn: &mut PgConnection, id: Uuid, d: &DatosRol) -> Result<(), RolError> {
    let (nombre, descripcion) = limpiar(d);
    if clave_desde_nombre(&nombre).is_none() {
        return Err(RolError::NombreInvalido);
    }
    let n =
        sqlx::query("UPDATE roles SET nombre = $2, descripcion = $3, activo = $4 WHERE id = $1")
            .bind(id)
            .bind(&nombre)
            .bind(&descripcion)
            .bind(d.activo)
            .execute(&mut *conn)
            .await?
            .rows_affected();
    if n == 0 {
        return Err(RolError::NoEncontrado);
    }
    sincronizar_permisos(conn, id, &d.permisos).await
}

pub async fn eliminar(conn: &mut PgConnection, id: Uuid) -> Result<(), RolError> {
    let n = sqlx::query("DELETE FROM roles WHERE id = $1")
        .bind(id)
        .execute(conn)
        .await?
        .rows_affected();
    if n == 0 {
        return Err(RolError::NoEncontrado);
    }
    Ok(())
}

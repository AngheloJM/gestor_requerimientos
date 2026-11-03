//! Roles y permisos configurables (fases 0 y 1 de docs/PLAN_ROLES.md).
//!
//!   GET    /permisos      catálogo (módulo → recurso → acción)
//!   GET    /roles         roles con sus permisos
//!   GET    /roles/{id}
//!   POST   /roles         {nombre, descripcion?, activo?, permisos: [{clave, alcance?}]} → 201
//!   PUT    /roles/{id}    mismo body: reemplaza datos y permisos
//!   DELETE /roles/{id}    → 204 (los roles de sistema no se borran)
//!
//! Autorización: RLS (administrar = rrhh_admin). Los permisos todavía NO se aplican al acceso.

use std::collections::{HashMap, HashSet};

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use db::{PgPool, roles::RolError};
use domain::{
    DatosRol, Permiso, PermisoAsignado,
    roles::{ALCANCES, Rol},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::{ApiError, ApiResult, tx_usuario},
};

#[derive(Debug, Serialize)]
pub struct PermisoDto {
    pub clave: String,
    pub modulo: String,
    pub recurso: String,
    pub recurso_nombre: String,
    pub accion: String,
    pub descripcion: String,
    pub admite_alcance: bool,
}

impl From<Permiso> for PermisoDto {
    fn from(p: Permiso) -> Self {
        Self {
            clave: p.clave,
            modulo: p.modulo,
            recurso: p.recurso,
            recurso_nombre: p.recurso_nombre,
            accion: p.accion,
            descripcion: p.descripcion,
            admite_alcance: p.admite_alcance,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct AsignadoDto {
    pub clave: String,
    #[serde(default = "alcance_todos")]
    pub alcance: String,
}

fn alcance_todos() -> String {
    "todos".into()
}

#[derive(Debug, Serialize)]
pub struct RolDto {
    pub id: Uuid,
    pub clave: String,
    pub nombre: String,
    pub descripcion: Option<String>,
    pub es_sistema: bool,
    pub activo: bool,
    pub permisos: Vec<AsignadoDto>,
    pub actualizado: String,
}

impl From<Rol> for RolDto {
    fn from(r: Rol) -> Self {
        Self {
            id: r.id,
            clave: r.clave,
            nombre: r.nombre,
            descripcion: r.descripcion,
            es_sistema: r.es_sistema,
            activo: r.activo,
            permisos: r
                .permisos
                .into_iter()
                .map(|p| AsignadoDto {
                    clave: p.clave,
                    alcance: p.alcance,
                })
                .collect(),
            actualizado: r
                .updated_at
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RolBody {
    pub nombre: String,
    pub descripcion: Option<String>,
    #[serde(default = "verdadero")]
    pub activo: bool,
    #[serde(default)]
    pub permisos: Vec<AsignadoDto>,
}

fn verdadero() -> bool {
    true
}

fn error_rol(e: RolError) -> ApiError {
    match e {
        RolError::NoEncontrado => ApiError::NotFound,
        RolError::NombreInvalido => {
            ApiError::Unprocessable("el nombre debe tener al menos 2 letras o números".into())
        }
        RolError::Duplicado => ApiError::Conflict("ya existe un rol con ese nombre".into()),
        RolError::Db(e) => e.into(),
    }
}

/// Valida el body contra el catálogo: claves existentes, sin repetir, alcance admitido.
fn validar(b: RolBody, catalogo: &[Permiso]) -> Result<DatosRol, ApiError> {
    let nombre = b.nombre.trim();
    if nombre.chars().count() > 80 {
        return Err(ApiError::Unprocessable(
            "el nombre admite hasta 80 caracteres".into(),
        ));
    }
    if b.descripcion
        .as_deref()
        .is_some_and(|d| d.chars().count() > 500)
    {
        return Err(ApiError::Unprocessable(
            "la descripción admite hasta 500 caracteres".into(),
        ));
    }
    let por_clave: HashMap<&str, &Permiso> =
        catalogo.iter().map(|p| (p.clave.as_str(), p)).collect();
    let mut vistos = HashSet::new();
    let mut permisos = Vec::with_capacity(b.permisos.len());
    for a in b.permisos {
        let Some(p) = por_clave.get(a.clave.as_str()) else {
            return Err(ApiError::Unprocessable(format!(
                "permiso desconocido: {}",
                a.clave
            )));
        };
        if !vistos.insert(a.clave.clone()) {
            return Err(ApiError::Unprocessable(format!(
                "permiso repetido: {}",
                a.clave
            )));
        }
        if !ALCANCES.contains(&a.alcance.as_str()) {
            return Err(ApiError::Unprocessable(format!(
                "alcance inválido: {}",
                a.alcance
            )));
        }
        if a.alcance != "todos" && !p.admite_alcance {
            return Err(ApiError::Unprocessable(format!(
                "el permiso {} no admite alcance limitado",
                a.clave
            )));
        }
        permisos.push(PermisoAsignado {
            clave: a.clave,
            alcance: a.alcance,
        });
    }
    Ok(DatosRol {
        nombre: nombre.to_owned(),
        descripcion: b.descripcion,
        activo: b.activo,
        permisos,
    })
}

pub async fn permisos(
    State(pool): State<PgPool>,
    user: AuthUser,
) -> ApiResult<Json<Vec<PermisoDto>>> {
    let mut tx = tx_usuario(&pool, &user).await?;
    let c = db::roles::catalogo(&mut tx).await?;
    Ok(Json(c.into_iter().map(Into::into).collect()))
}

pub async fn listar(State(pool): State<PgPool>, user: AuthUser) -> ApiResult<Json<Vec<RolDto>>> {
    let mut tx = tx_usuario(&pool, &user).await?;
    let v = db::roles::listar(&mut tx).await?;
    Ok(Json(v.into_iter().map(Into::into).collect()))
}

pub async fn obtener(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<RolDto>> {
    let mut tx = tx_usuario(&pool, &user).await?;
    let r = db::roles::obtener(&mut tx, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(r.into()))
}

pub async fn crear(
    State(pool): State<PgPool>,
    user: AuthUser,
    Json(b): Json<RolBody>,
) -> ApiResult<(StatusCode, Json<RolDto>)> {
    let mut tx = tx_usuario(&pool, &user).await?;
    let datos = validar(b, &db::roles::catalogo(&mut tx).await?)?;
    let id = db::roles::crear(&mut tx, &datos).await.map_err(error_rol)?;
    let r = db::roles::obtener(&mut tx, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    tx.commit().await?;
    tracing::info!(usuario_id = %user.user_id, rol = %r.clave, permisos = r.permisos.len(), "rol creado");
    Ok((StatusCode::CREATED, Json(r.into())))
}

pub async fn actualizar(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(b): Json<RolBody>,
) -> ApiResult<Json<RolDto>> {
    let mut tx = tx_usuario(&pool, &user).await?;
    let datos = validar(b, &db::roles::catalogo(&mut tx).await?)?;
    db::roles::actualizar(&mut tx, id, &datos)
        .await
        .map_err(error_rol)?;
    let r = db::roles::obtener(&mut tx, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    tx.commit().await?;
    tracing::info!(usuario_id = %user.user_id, rol = %r.clave, permisos = r.permisos.len(), "rol actualizado");
    Ok(Json(r.into()))
}

pub async fn eliminar(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let mut tx = tx_usuario(&pool, &user).await?;
    db::roles::eliminar(&mut tx, id).await.map_err(error_rol)?;
    tx.commit().await?;
    tracing::info!(usuario_id = %user.user_id, rol_id = %id, "rol eliminado");
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cat() -> Vec<Permiso> {
        let p = |clave: &str, alcance| Permiso {
            clave: clave.into(),
            modulo: "M".into(),
            recurso: "r".into(),
            recurso_nombre: "R".into(),
            accion: "a".into(),
            descripcion: "d".into(),
            admite_alcance: alcance,
            orden: 1,
        };
        vec![p("requerimientos.ver", true), p("clientes.ver", false)]
    }

    fn body(permisos: serde_json::Value) -> RolBody {
        serde_json::from_value(serde_json::json!({"nombre": " Gerente ", "permisos": permisos}))
            .unwrap()
    }

    #[test]
    fn valida_contra_el_catalogo() {
        let d = validar(
            body(serde_json::json!([{"clave": "requerimientos.ver", "alcance": "propios"}, {"clave": "clientes.ver"}])),
            &cat(),
        )
        .unwrap();
        assert_eq!(d.nombre, "Gerente");
        assert!(d.activo);
        assert_eq!(d.permisos[1].alcance, "todos", "alcance por defecto");

        for (permisos, error) in [
            (serde_json::json!([{"clave": "x.ver"}]), "desconocido"),
            (
                serde_json::json!([{"clave": "clientes.ver"}, {"clave": "clientes.ver"}]),
                "repetido",
            ),
            (
                serde_json::json!([{"clave": "clientes.ver", "alcance": "propios"}]),
                "no admite",
            ),
            (
                serde_json::json!([{"clave": "requerimientos.ver", "alcance": "otro"}]),
                "alcance inválido",
            ),
        ] {
            match validar(body(permisos), &cat()) {
                Err(ApiError::Unprocessable(m)) => assert!(m.contains(error), "{m}"),
                otro => panic!("esperaba 422 con '{error}', llegó {otro:?}"),
            }
        }
    }
}

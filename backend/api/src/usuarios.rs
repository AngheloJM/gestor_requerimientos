//! Administración de usuarios (pantalla "Usuarios"; solo RR.HH. puede editar: RLS).
//!
//!   GET /usuarios            lista con cargo, superior, rol, estado y servicios asignados
//!   GET /usuarios/opciones   cargos, servicios activos y roles admitidos
//!   PUT /usuarios/{id}       {cargo_id, division, region, inmediato_superior_id, rol, activo, servicios}
//!
//! Alta y baja de usuarios: no disponibles. `usuarios.id` es FK a `auth.users` (Supabase Auth),
//! así que crear un usuario requiere una decisión (ver docs/PENDIENTES.md).

use axum::{
    Json,
    extract::{Path, State},
};
use db::{
    PgPool,
    usuarios::{DatosUsuario, EditarUsuarioError, UsuarioAdmin},
};
use domain::Rol;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    dto::RefDto,
    error::{ApiError, ApiResult, tx_usuario},
};

#[derive(Debug, Serialize)]
pub struct UsuarioDto {
    pub id: Uuid,
    pub nombre: String,
    pub cargo: Option<RefDto>,
    pub division: Option<String>,
    pub region: Option<String>,
    pub inmediato_superior: Option<RefDto>,
    pub rol: String,
    pub activo: bool,
    pub servicios: Vec<RefDto>,
}

impl From<UsuarioAdmin> for UsuarioDto {
    fn from(u: UsuarioAdmin) -> Self {
        Self {
            id: u.id,
            nombre: u.nombre,
            cargo: u.cargo_id.map(|id| RefDto::new(id, u.cargo_nombre)),
            division: u.division,
            region: u.region,
            inmediato_superior: u
                .inmediato_superior_id
                .map(|id| RefDto::new(id, u.superior_nombre)),
            rol: u.rol,
            activo: u.activo,
            servicios: u
                .servicios
                .into_iter()
                .map(|(id, n)| RefDto::new(id, Some(n)))
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct OpcionesUsuarioDto {
    pub cargos: Vec<RefDto>,
    pub servicios: Vec<RefDto>,
    /// Valores del CHECK `usuarios_rol_check` (un usuario = un rol).
    pub roles: Vec<&'static str>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsuarioBody {
    pub cargo_id: Option<Uuid>,
    pub division: Option<String>,
    pub region: Option<String>,
    pub inmediato_superior_id: Option<Uuid>,
    pub rol: String,
    pub activo: bool,
    #[serde(default)]
    pub servicios: Vec<Uuid>,
}

/// Validaciones de integridad previas a la base.
fn validar(id: Uuid, b: UsuarioBody) -> Result<DatosUsuario, ApiError> {
    if b.rol.parse::<Rol>().is_err() {
        return Err(ApiError::Unprocessable(format!("rol inválido: {}", b.rol)));
    }
    if b.inmediato_superior_id == Some(id) {
        return Err(ApiError::Unprocessable(
            "un usuario no puede ser su propio inmediato superior".into(),
        ));
    }
    for (campo, v) in [("división", &b.division), ("región", &b.region)] {
        if v.as_deref().is_some_and(|s| s.chars().count() > 120) {
            return Err(ApiError::Unprocessable(format!(
                "{campo}: hasta 120 caracteres"
            )));
        }
    }
    let mut servicios = b.servicios;
    servicios.sort();
    servicios.dedup();
    if servicios.len() > 500 {
        return Err(ApiError::Unprocessable("demasiados servicios".into()));
    }
    Ok(DatosUsuario {
        cargo_id: b.cargo_id,
        division: b.division,
        region: b.region,
        inmediato_superior_id: b.inmediato_superior_id,
        rol: b.rol,
        activo: b.activo,
        servicios,
    })
}

pub async fn listar(
    State(pool): State<PgPool>,
    user: AuthUser,
) -> ApiResult<Json<Vec<UsuarioDto>>> {
    let mut tx = tx_usuario(&pool, &user).await?;
    let v = db::usuarios::listar_admin(&mut tx).await?;
    Ok(Json(v.into_iter().map(Into::into).collect()))
}

pub async fn opciones(
    State(pool): State<PgPool>,
    user: AuthUser,
) -> ApiResult<Json<OpcionesUsuarioDto>> {
    let mut tx = tx_usuario(&pool, &user).await?;
    let (cargos, servicios) = db::usuarios::opciones_admin(&mut tx).await?;
    let a_ref = |v: Vec<(Uuid, String)>| {
        v.into_iter()
            .map(|(id, n)| RefDto::new(id, Some(n)))
            .collect()
    };
    Ok(Json(OpcionesUsuarioDto {
        cargos: a_ref(cargos),
        servicios: a_ref(servicios),
        roles: Rol::TODOS.iter().map(|r| r.as_str()).collect(),
    }))
}

pub async fn actualizar(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(b): Json<UsuarioBody>,
) -> ApiResult<Json<UsuarioDto>> {
    let datos = validar(id, b)?;
    let mut tx = tx_usuario(&pool, &user).await?;
    db::usuarios::actualizar_admin(&mut tx, id, &datos)
        .await
        .map_err(|e| match e {
            // RLS: UPDATE sin filas = no existe o no es RR.HH.
            EditarUsuarioError::NoEncontrado => ApiError::NotFound,
            EditarUsuarioError::Db(e) => e.into(),
        })?;
    let u = db::usuarios::listar_admin(&mut tx)
        .await?
        .into_iter()
        .find(|u| u.id == id)
        .ok_or(ApiError::NotFound)?;
    tx.commit().await?;
    tracing::info!(usuario_id = %user.user_id, editado = %id, rol = %u.rol, activo = u.activo, "usuario editado");
    Ok(Json(u.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(v: serde_json::Value) -> UsuarioBody {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn valida_rol_superior_y_deduplica_servicios() {
        let id = Uuid::new_v4();
        let s = Uuid::new_v4();
        let d = validar(
            id,
            body(serde_json::json!({"rol": "estandar", "activo": true, "servicios": [s, s]})),
        )
        .unwrap();
        assert_eq!(d.servicios, [s]);
        assert!(matches!(
            validar(id, body(serde_json::json!({"rol": "gerente", "activo": true}))),
            Err(ApiError::Unprocessable(m)) if m.contains("rol inválido")
        ));
        assert!(matches!(
            validar(id, body(serde_json::json!({"rol": "estandar", "activo": true, "inmediato_superior_id": id}))),
            Err(ApiError::Unprocessable(m)) if m.contains("propio inmediato superior")
        ));
    }
}

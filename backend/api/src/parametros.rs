//! Parámetros salariales (salario mínimo nacional, recargo nocturno, …).
//!
//!   GET  /parametros-salariales   vigentes hoy (zona del negocio) + historial completo
//!   POST /parametros-salariales   {parametro, valor, vigente_desde} → 201 (solo RR.HH.: RLS)
//!
//! Cada cambio agrega una fila nueva con su vigencia; nunca se pisa el valor anterior.

use axum::{Json, extract::State, http::StatusCode};
use chrono::NaiveDate;
use db::{PgPool, parametros::ValorParametro};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::{ApiError, ApiResult, tx_usuario},
    tablero::hoy_negocio,
};

#[derive(Debug, Serialize)]
pub struct ValorDto {
    pub id: Uuid,
    pub parametro: String,
    pub valor: Decimal,
    pub vigente_desde: NaiveDate,
    pub registrado: String,
}

impl From<&ValorParametro> for ValorDto {
    fn from(v: &ValorParametro) -> Self {
        Self {
            id: v.id,
            parametro: v.parametro.clone(),
            valor: v.valor.normalize(),
            vigente_desde: v.vigente_desde,
            registrado: v
                .created_at
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ParametrosDto {
    pub fecha: NaiveDate,
    pub vigentes: Vec<ValorDto>,
    pub historial: Vec<ValorDto>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NuevoValor {
    pub parametro: String,
    pub valor: Decimal,
    pub vigente_desde: NaiveDate,
}

fn clave_valida(p: &str) -> bool {
    (2..=60).contains(&p.len())
        && p.starts_with(|c: char| c.is_ascii_lowercase())
        && p.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

pub async fn listar(State(pool): State<PgPool>, user: AuthUser) -> ApiResult<Json<ParametrosDto>> {
    let mut tx = tx_usuario(&pool, &user).await?;
    let h = db::parametros::historial(&mut tx).await?;
    let fecha = hoy_negocio();
    Ok(Json(ParametrosDto {
        fecha,
        vigentes: db::parametros::vigentes(&h, fecha)
            .into_iter()
            .map(Into::into)
            .collect(),
        historial: h.iter().map(Into::into).collect(),
    }))
}

pub async fn registrar(
    State(pool): State<PgPool>,
    user: AuthUser,
    Json(b): Json<NuevoValor>,
) -> ApiResult<StatusCode> {
    if !clave_valida(&b.parametro) {
        return Err(ApiError::Unprocessable(
            "parámetro inválido (minúsculas, números y _)".into(),
        ));
    }
    if b.valor.is_sign_negative() {
        return Err(ApiError::Unprocessable(
            "el valor no puede ser negativo".into(),
        ));
    }
    let mut tx = tx_usuario(&pool, &user).await?;
    db::parametros::registrar(&mut tx, &b.parametro, b.valor, b.vigente_desde)
        .await
        .map_err(|e| {
            if e.es_duplicado() {
                ApiError::Conflict(
                    "ya hay un valor de ese parámetro con esa fecha de vigencia".into(),
                )
            } else {
                e.into()
            }
        })?;
    tx.commit().await?;
    tracing::info!(usuario_id = %user.user_id, parametro = %b.parametro, valor = %b.valor, desde = %b.vigente_desde, "parámetro salarial registrado");
    Ok(StatusCode::CREATED)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claves_validas() {
        assert!(clave_valida("salario_minimo_nacional"));
        assert!(clave_valida("recargo_nocturno_pct"));
        for mala in ["", "a", "Salario", "1x", "con espacio", "ñ"] {
            assert!(!clave_valida(mala), "{mala}");
        }
    }
}

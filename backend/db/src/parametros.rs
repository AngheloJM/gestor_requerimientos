//! Parámetros salariales globales (`parametros_salariales_historial`): cada cambio agrega una
//! fila con su fecha de vigencia; el valor vigente a una fecha es el de mayor
//! `vigente_desde <= fecha`. Lectura: todos; alta: RR.HH. (RLS).

use chrono::{DateTime, NaiveDate, Utc};
use domain::Decimal;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::DbError;

/// Clave del salario mínimo nacional (verificada en producción).
pub const SALARIO_MINIMO_NACIONAL: &str = "salario_minimo_nacional";

#[derive(Debug, Clone, FromRow)]
pub struct ValorParametro {
    pub id: Uuid,
    pub parametro: String,
    pub valor: Decimal,
    pub vigente_desde: NaiveDate,
    pub creado_por: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

/// Historial completo, por parámetro y del más reciente al más antiguo.
pub async fn historial(conn: &mut PgConnection) -> Result<Vec<ValorParametro>, DbError> {
    Ok(sqlx::query_as(
        "SELECT id, parametro::text AS parametro, valor::numeric AS valor, vigente_desde,
                creado_por, created_at
         FROM parametros_salariales_historial
         ORDER BY parametro, vigente_desde DESC, created_at DESC",
    )
    .fetch_all(conn)
    .await?)
}

/// Valor vigente de cada parámetro a la fecha dada.
pub fn vigentes(historial: &[ValorParametro], fecha: NaiveDate) -> Vec<&ValorParametro> {
    let mut out: Vec<&ValorParametro> = Vec::new();
    for v in historial.iter().filter(|v| v.vigente_desde <= fecha) {
        match out.iter_mut().find(|x| x.parametro == v.parametro) {
            Some(x) if x.vigente_desde < v.vigente_desde => *x = v,
            Some(_) => {}
            None => out.push(v),
        }
    }
    out
}

/// Registra un nuevo valor (no modifica los anteriores). `creado_por` = usuario actual.
pub async fn registrar(
    conn: &mut PgConnection,
    parametro: &str,
    valor: Decimal,
    vigente_desde: NaiveDate,
) -> Result<Uuid, DbError> {
    Ok(sqlx::query_scalar(
        "INSERT INTO parametros_salariales_historial (parametro, valor, vigente_desde, creado_por)
         VALUES ($1, $2, $3, auth.uid()) RETURNING id",
    )
    .bind(parametro)
    .bind(valor)
    .bind(vigente_desde)
    .fetch_one(conn)
    .await?)
}

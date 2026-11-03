//! Movimientos del almanaque desde la función de producción `leer_eventos_almanaque`
//! (la misma que usa la aplicación original; verificada el 2026-11-03).
//!
//! - Acceso: lo decide la función (RR.HH. todo; costos y estándar sus servicios asignados;
//!   otros roles → 42501 → 403).
//! - Regla propia de este proyecto: **solo servicios activos**. La función no filtra
//!   `servicios.activo`, así que cada evento se cruza por folio con su requerimiento y se
//!   conserva solo si su servicio está activo (o no tiene servicio). Si RLS no deja ver el
//!   requerimiento o el servicio, el evento se descarta (falla cerrada).

use chrono::{Days, NaiveDate};
use domain::{Movimiento, Periodo, TipoMovimiento};
use sqlx::{FromRow, PgConnection};

use crate::DbError;

/// $1 = desde, $2 = hasta (ambos inclusive, como la función).
const QUERY_EVENTOS: &str = r#"
SELECT ev.tipo_evento::text AS tipo, ev.fecha, ev.cliente::text AS cliente,
       ev.servicio::text AS servicio, ev.cargo::text AS cargo, ev.cantidad::int8 AS cantidad,
       ev.folio::text AS folio, ev.estado_entrega::text AS estado_entrega
FROM public.leer_eventos_almanaque($1, $2) AS ev
WHERE EXISTS (
    SELECT 1 FROM requerimientos r
    LEFT JOIN servicios s ON s.id = r.servicio_id
    WHERE r.folio = ev.folio AND (r.servicio_id IS NULL OR s.activo IS TRUE)
)
"#;

#[derive(FromRow)]
struct EventoRow {
    tipo: String,
    fecha: NaiveDate,
    cliente: Option<String>,
    servicio: Option<String>,
    cargo: Option<String>,
    cantidad: Option<i64>,
    folio: Option<String>,
    estado_entrega: Option<String>,
}

pub async fn listar_movimientos(
    conn: &mut PgConnection,
    periodo: Periodo,
) -> Result<Vec<Movimiento>, DbError> {
    let hasta = periodo
        .hasta()
        .checked_sub_days(Days::new(1))
        .expect("el periodo tiene al menos un día");
    let rows: Vec<EventoRow> = sqlx::query_as(QUERY_EVENTOS)
        .bind(periodo.desde())
        .bind(hasta)
        .fetch_all(conn)
        .await?;

    Ok(rows
        .into_iter()
        .filter_map(|r| {
            // Un código desconocido (si la función cambia) se descarta y se registra.
            let Some(tipo) = TipoMovimiento::desde_codigo(&r.tipo) else {
                tracing::warn!(tipo = %r.tipo, "tipo de evento de almanaque desconocido");
                return None;
            };
            Some(Movimiento {
                tipo,
                fecha: r.fecha,
                cliente: r.cliente,
                servicio: r.servicio,
                cargo: r.cargo,
                cantidad: r.cantidad,
                folio: r.folio,
                estado_entrega: r.estado_entrega,
            })
        })
        .collect())
}

//! Movimientos del almanaque: combina `requerimientos` (entradas / salidas) y
//! `requerimiento_entregas` (entregas / rectificaciones, según `corrige_entrega_id`).
//!
//! - Se excluyen los de servicios inactivos (`servicios.activo = true`); los requerimientos
//!   sin servicio (`servicio_id` NULL) se incluyen con servicio vacío.
//! - No se filtra por `requerimientos.estado` ni por la decisión de la entrega.
//!   TODO(negocio): qué estados de requerimiento / decisiones de entrega cuentan.
//! - TODO(negocio): rol de `requerimiento_entregas.tipo` en la clasificación (hoy solo se usa
//!   `corrige_entrega_id` para distinguir entrega de rectificación).
//! - TODO(técnico): si las fechas son `timestamptz`, el corte del día depende de la zona
//!   horaria de la sesión; hoy se castean a `date` tal cual.

use chrono::NaiveDate;
use domain::{Movimiento, Periodo, TipoMovimiento};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::{DbError, cantidad_desde_db};

/// $1 = desde (inclusive), $2 = hasta (exclusivo).
const QUERY_MOVIMIENTOS: &str = r#"
WITH req AS (
    SELECT r.*, s.nombre::text AS servicio_nombre, c.nombre::text AS cargo_nombre
    FROM requerimientos r
    LEFT JOIN servicios s ON s.id = r.servicio_id
    LEFT JOIN cargos    c ON c.id = r.cargo_id
    WHERE r.servicio_id IS NULL OR s.activo IS TRUE
)
SELECT 'entrada' AS tipo, fecha_inicio_operacion::date AS fecha,
       servicio_id, servicio_nombre, cargo_id, cargo_nombre,
       cantidad_personas::int8 AS cantidad
FROM req
WHERE fecha_inicio_operacion >= $1 AND fecha_inicio_operacion < $2
UNION ALL
SELECT 'salida', fecha_salida::date,
       servicio_id, servicio_nombre, cargo_id, cargo_nombre,
       cantidad_personas::int8
FROM req
WHERE fecha_salida >= $1 AND fecha_salida < $2
UNION ALL
SELECT CASE WHEN e.corrige_entrega_id IS NULL THEN 'entrega' ELSE 'rectificacion' END,
       e.fecha_entrega::date,
       req.servicio_id, req.servicio_nombre, e.cargo_id, c.nombre::text,
       e.cantidad::int8
FROM requerimiento_entregas e
JOIN req          ON req.id = e.requerimiento_id
LEFT JOIN cargos c ON c.id = e.cargo_id
WHERE e.fecha_entrega >= $1 AND e.fecha_entrega < $2
"#;

#[derive(FromRow)]
struct MovRow {
    tipo: String,
    fecha: NaiveDate,
    servicio_id: Option<Uuid>,
    servicio_nombre: Option<String>,
    cargo_id: Uuid,
    cargo_nombre: Option<String>,
    cantidad: Option<i64>,
}

pub async fn listar_movimientos(
    conn: &mut PgConnection,
    periodo: Periodo,
) -> Result<Vec<Movimiento>, DbError> {
    let rows: Vec<MovRow> = sqlx::query_as(QUERY_MOVIMIENTOS)
        .bind(periodo.desde())
        .bind(periodo.hasta())
        .fetch_all(conn)
        .await?;

    rows.into_iter()
        .map(|r| {
            Ok(Movimiento {
                tipo: TipoMovimiento::desde_codigo(&r.tipo).expect("tipo generado por la query"),
                fecha: r.fecha,
                servicio_id: r.servicio_id.map(Into::into),
                servicio_nombre: r.servicio_nombre,
                cargo_id: r.cargo_id.into(),
                cargo_nombre: r.cargo_nombre,
                cantidad: r
                    .cantidad
                    .map(|c| cantidad_desde_db("cantidad", c))
                    .transpose()?,
            })
        })
        .collect()
}

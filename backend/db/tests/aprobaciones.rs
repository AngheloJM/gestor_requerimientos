mod comun;

use chrono::NaiveDate;
use comun::*;
use db::aprobaciones::{
    ActualizarError, EdicionFechaFin, actualizar_fecha_fin, historial, listar, obtener,
};
use domain::{DomainError, RequerimientoId, UsuarioId};
use sqlx::PgConnection;

const REQ_81: &str = "00000000-0000-0000-0000-000000000081";
/// Servicio Soporte (asignado a ESTANDAR).
const REQ_84: &str = "00000000-0000-0000-0000-000000000084";

fn req81() -> RequerimientoId {
    RequerimientoId(uid(REQ_81))
}

fn d(m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, m, day).unwrap()
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(migrations = false, fixtures("supabase_subset", "rls_test", "datos"))]
async fn listar_campos_reales_y_solo_activos(pool: db::PgPool) {
    let mut tx = como(&pool, RRHH).await;
    let v = listar(&mut tx).await.unwrap();
    // 6 requerimientos, menos el del servicio inactivo.
    assert_eq!(v.len(), 5);
    let r = v.iter().find(|a| a.id == req81()).unwrap();
    assert_eq!(r.fecha_inicio, d(11, 1));
    assert_eq!(r.fecha_fin, Some(d(11, 20)));
    assert_eq!(r.cantidad, Some(3));
    assert!(
        r.ultima_modificacion.is_some(),
        "última edición de fecha_fin en el historial"
    );
    let sin_cantidad = v
        .iter()
        .find(|a| a.cargo_nombre.as_deref() == Some("Agente"))
        .unwrap();
    assert_eq!(sin_cantidad.cantidad, None);
}

async fn edicion<'a>(
    conn: &mut PgConnection,
    id: RequerimientoId,
    nueva: Option<NaiveDate>,
    glosa: &'a str,
) -> EdicionFechaFin<'a> {
    let version = obtener(conn, id)
        .await
        .unwrap()
        .map(|a| a.version)
        .unwrap_or_default();
    EdicionFechaFin {
        nueva,
        version,
        glosa,
        categoria_desvio: None,
        detalle_desvio: None,
    }
}

// Los tests de edición cargan `p2_edicion`: copia textual de la función, triggers y
// políticas de producción (editar_requerimiento_con_historial, p2_acceso, etc.).

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(
    migrations = false,
    fixtures("supabase_subset", "rls_test", "datos", "p2_edicion")
)]
async fn editar_registra_historial_una_sola_vez_via_trigger(pool: db::PgPool) {
    let mut tx = como(&pool, RRHH).await;
    let e = edicion(&mut tx, req81(), Some(d(12, 31)), "extensión del servicio").await;
    let a = actualizar_fecha_fin(&mut tx, req81(), e).await.unwrap();
    assert_eq!(a.fecha_fin, Some(d(12, 31)));
    assert_ne!(a.version, e.version, "updated_at avanza");
    tx.commit().await.unwrap();

    let mut tx = como(&pool, RRHH).await;
    let h = historial(&mut tx, req81()).await.unwrap().unwrap();
    let resumen: Vec<_> = h
        .iter()
        .map(|e| {
            (
                e.campo.as_str(),
                e.valor_anterior.as_deref(),
                e.valor_nuevo.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        resumen,
        [
            ("estado", Some("a"), Some("b")),
            ("fecha_fin", Some("2026-11-15"), Some("2026-11-20")),
            ("fecha_fin", Some("2026-11-20"), Some("2026-12-31")),
        ],
        "una sola fila nueva (la escribe el trigger), sin duplicados"
    );
    let ultimo = h.last().unwrap();
    assert_eq!(ultimo.usuario_id, Some(UsuarioId(uid(RRHH))));
    assert_eq!(ultimo.glosa.as_deref(), Some("extensión del servicio"));
    assert!(ultimo.grupo_edicion.is_some());
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(
    migrations = false,
    fixtures("supabase_subset", "rls_test", "datos", "p2_edicion")
)]
async fn mismo_valor_no_escribe_historial(pool: db::PgPool) {
    let mut tx = como(&pool, RRHH).await;
    let e = edicion(&mut tx, req81(), Some(d(11, 20)), "sin cambios").await;
    actualizar_fecha_fin(&mut tx, req81(), e).await.unwrap();
    assert_eq!(historial(&mut tx, req81()).await.unwrap().unwrap().len(), 2);
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(
    migrations = false,
    fixtures("supabase_subset", "rls_test", "datos", "p2_edicion")
)]
async fn reglas_de_la_base(pool: db::PgPool) {
    // Glosa obligatoria (3 a 2000 caracteres).
    let mut tx = como(&pool, RRHH).await;
    let e = edicion(&mut tx, req81(), Some(d(12, 31)), "  ok ").await;
    assert!(matches!(
        actualizar_fecha_fin(&mut tx, req81(), e).await,
        Err(ActualizarError::Regla(m)) if m.starts_with("Glosa obligatoria")
    ));
    drop(tx);

    // Versión desactualizada → conflicto.
    let mut tx = como(&pool, RRHH).await;
    let mut e = edicion(&mut tx, req81(), Some(d(12, 31)), "extensión").await;
    e.version -= chrono::Duration::seconds(1);
    assert!(matches!(
        actualizar_fecha_fin(&mut tx, req81(), e).await,
        Err(ActualizarError::Conflicto)
    ));
    drop(tx);

    // Categoría de desvío fuera del catálogo de la base.
    let mut tx = como(&pool, RRHH).await;
    let mut e = edicion(&mut tx, req81(), Some(d(12, 31)), "extensión").await;
    e.categoria_desvio = Some("otra");
    e.detalle_desvio = Some("detalle");
    assert!(matches!(
        actualizar_fecha_fin(&mut tx, req81(), e).await,
        Err(ActualizarError::Regla(m)) if m == "Categoría de desvío inválida"
    ));
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(
    migrations = false,
    fixtures("supabase_subset", "rls_test", "datos", "p2_edicion")
)]
async fn validacion_y_permisos(pool: db::PgPool) {
    let mut tx = como(&pool, RRHH).await;
    let e = edicion(&mut tx, req81(), Some(d(10, 1)), "antes del inicio").await;
    assert!(matches!(
        actualizar_fecha_fin(&mut tx, req81(), e).await,
        Err(ActualizarError::Validacion(
            DomainError::RangoFechasInvalido
        ))
    ));
    drop(tx);

    // COSTOS ve el requerimiento (Call center) pero p2_acceso no le permite editarlo.
    let mut tx = como(&pool, COSTOS).await;
    let e = edicion(&mut tx, req81(), None, "quitar fecha").await;
    assert!(matches!(
        actualizar_fecha_fin(&mut tx, req81(), e).await,
        Err(ActualizarError::SinPermiso)
    ));
    drop(tx);

    // SIN_SERVICIOS ni siquiera lo ve.
    let mut tx = como(&pool, SIN_SERVICIOS).await;
    let e = edicion(&mut tx, req81(), None, "quitar fecha").await;
    assert!(matches!(
        actualizar_fecha_fin(&mut tx, req81(), e).await,
        Err(ActualizarError::NoEncontrado)
    ));
    assert!(historial(&mut tx, req81()).await.unwrap().is_none());
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(
    migrations = false,
    fixtures("supabase_subset", "rls_test", "datos", "p2_edicion")
)]
async fn estandar_con_servicio_asignado_edita_fecha_fin(pool: db::PgPool) {
    // Regla de producción (p2_acceso): `estandar` con el servicio asignado puede editar.
    let req84 = RequerimientoId(uid(REQ_84));
    let mut tx = como(&pool, ESTANDAR).await;
    let e = edicion(&mut tx, req84, Some(d(12, 1)), "cierre del servicio").await;
    let a = actualizar_fecha_fin(&mut tx, req84, e).await.unwrap();
    assert_eq!(a.fecha_fin, Some(d(12, 1)));
}

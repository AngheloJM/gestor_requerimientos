mod comun;

use chrono::NaiveDate;
use comun::*;
use db::aprobaciones::{ActualizarError, actualizar_fecha_fin, historial, listar};
use domain::{DomainError, RequerimientoId, UsuarioId};

const REQ_81: &str = "00000000-0000-0000-0000-000000000081";

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

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(migrations = false, fixtures("supabase_subset", "rls_test", "datos"))]
async fn editar_escribe_requerimiento_historial_y_devuelve_historial_completo(pool: db::PgPool) {
    let mut tx = como(&pool, RRHH).await;
    let a = actualizar_fecha_fin(
        &mut tx,
        req81(),
        Some(d(12, 31)),
        UsuarioId(uid(RRHH)),
        Some("extensión"),
    )
    .await
    .unwrap();
    assert_eq!(a.fecha_fin, Some(d(12, 31)));
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
        "historial completo, ordenado por created_at"
    );
    let ultimo = h.last().unwrap();
    assert_eq!(ultimo.usuario_id, Some(UsuarioId(uid(RRHH))));
    assert_eq!(ultimo.glosa.as_deref(), Some("extensión"));
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(migrations = false, fixtures("supabase_subset", "rls_test", "datos"))]
async fn mismo_valor_no_escribe_historial(pool: db::PgPool) {
    let mut tx = como(&pool, RRHH).await;
    actualizar_fecha_fin(
        &mut tx,
        req81(),
        Some(d(11, 20)),
        UsuarioId(uid(RRHH)),
        None,
    )
    .await
    .unwrap();
    assert_eq!(historial(&mut tx, req81()).await.unwrap().unwrap().len(), 2);
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(migrations = false, fixtures("supabase_subset", "rls_test", "datos"))]
async fn validacion_y_permisos_por_rls(pool: db::PgPool) {
    let mut tx = como(&pool, RRHH).await;
    assert!(matches!(
        actualizar_fecha_fin(&mut tx, req81(), Some(d(10, 1)), UsuarioId(uid(RRHH)), None).await,
        Err(ActualizarError::Validacion(
            DomainError::RangoFechasInvalido
        ))
    ));
    drop(tx);

    // COSTOS ve el requerimiento (Call center) pero RLS no le permite modificarlo.
    let mut tx = como(&pool, COSTOS).await;
    assert!(matches!(
        actualizar_fecha_fin(&mut tx, req81(), None, UsuarioId(uid(COSTOS)), None).await,
        Err(ActualizarError::SinPermiso)
    ));
    drop(tx);

    // SIN_SERVICIOS ni siquiera lo ve.
    let mut tx = como(&pool, SIN_SERVICIOS).await;
    assert!(matches!(
        actualizar_fecha_fin(&mut tx, req81(), None, UsuarioId(uid(SIN_SERVICIOS)), None).await,
        Err(ActualizarError::NoEncontrado)
    ));
    assert!(historial(&mut tx, req81()).await.unwrap().is_none());
}

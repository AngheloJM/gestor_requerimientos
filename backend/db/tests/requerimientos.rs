mod comun;

use comun::*;
use db::requerimientos::listar;
use domain::UsuarioId;

const REQ_81: &str = "00000000-0000-0000-0000-000000000081";
const REQ_84: &str = "00000000-0000-0000-0000-000000000084";

/// Asigna solicitantes y fechas de creación (como superusuario, sin RLS).
async fn preparar(pool: &db::PgPool) {
    sqlx::query(
        "UPDATE requerimientos SET created_at = '2026-10-01'::timestamptz + (numero || ' days')::interval",
    )
    .execute(pool)
    .await
    .unwrap();
    for (usuario, req) in [(ESTANDAR, REQ_84), (RRHH, REQ_81)] {
        sqlx::query("UPDATE requerimientos SET solicitante_id = $1 WHERE id = $2")
            .bind(uid(usuario))
            .bind(uid(req))
            .execute(pool)
            .await
            .unwrap();
    }
}

async fn ids(pool: &db::PgPool, usuario: &str, mios: bool) -> Vec<String> {
    let mut tx = como(pool, usuario).await;
    let filtro = mios.then(|| UsuarioId(uid(usuario)));
    listar(&mut tx, filtro)
        .await
        .unwrap()
        .into_iter()
        .map(|r| uuid::Uuid::from(r.id).to_string())
        .collect()
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(migrations = false, fixtures("supabase_subset", "rls_test", "datos"))]
async fn rrhh_ve_todos_los_activos_del_mas_reciente_al_mas_antiguo(pool: db::PgPool) {
    preparar(&pool).await;
    let v = ids(&pool, RRHH, false).await;
    assert_eq!(
        v.len(),
        5,
        "6 requerimientos menos el del servicio inactivo"
    );
    assert!(
        !v.iter().any(|id| id.ends_with("85")),
        "servicio inactivo excluido"
    );
    assert!(v.first().unwrap().ends_with("86"), "más reciente primero");
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(migrations = false, fixtures("supabase_subset", "rls_test", "datos"))]
async fn mios_filtra_por_solicitante_dentro_de_lo_visible(pool: db::PgPool) {
    preparar(&pool).await;
    assert_eq!(ids(&pool, RRHH, true).await, [REQ_81]);
    assert_eq!(ids(&pool, ESTANDAR, true).await, [REQ_84]);
    // ESTANDAR solo ve su servicio (RLS); COSTOS ve Call center pero no pidió nada.
    assert_eq!(ids(&pool, ESTANDAR, false).await, [REQ_84]);
    assert_eq!(ids(&pool, COSTOS, false).await.len(), 3);
    assert!(ids(&pool, COSTOS, true).await.is_empty());
}

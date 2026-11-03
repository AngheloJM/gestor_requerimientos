//! La autorización la decide RLS en Postgres: aquí se verifica que la identidad se propaga
//! y que no se filtra entre requests que comparten conexión.

mod comun;

use comun::*;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(migrations = false, fixtures("supabase_subset", "rls_test", "datos"))]
async fn identidad_se_propaga_a_postgres(pool: db::PgPool) {
    let mut tx = como(&pool, COSTOS).await;
    let (rol, uid_pg): (String, Option<uuid::Uuid>) =
        sqlx::query_as("SELECT current_user::text, auth.uid()")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert_eq!(rol, "authenticated");
    assert_eq!(uid_pg, Some(uid(COSTOS)));
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(migrations = false, fixtures("supabase_subset", "rls_test", "datos"))]
async fn rls_filtra_por_usuario(pool: db::PgPool) {
    async fn servicios_visibles(pool: &db::PgPool, u: &str) -> i64 {
        let mut tx = como(pool, u).await;
        sqlx::query_scalar("SELECT count(*) FROM servicios")
            .fetch_one(&mut *tx)
            .await
            .unwrap()
    }
    assert_eq!(servicios_visibles(&pool, RRHH).await, 6);
    assert_eq!(servicios_visibles(&pool, COSTOS).await, 3);
    assert_eq!(servicios_visibles(&pool, ESTANDAR).await, 1);
    assert_eq!(servicios_visibles(&pool, SIN_SERVICIOS).await, 0);
}

/// Con UNA sola conexión en el pool: tras el request del usuario A (commit o rollback), la
/// conexión vuelve sin rol ni claims, y el usuario B no hereda nada.
#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(migrations = false, fixtures("supabase_subset", "rls_test", "datos"))]
async fn set_local_no_se_filtra_entre_requests(_o: PgPoolOptions, conn: PgConnectOptions) {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_with(conn)
        .await
        .unwrap();

    let tx = como(&pool, RRHH).await;
    tx.commit().await.unwrap();

    let (rol, claims): (String, Option<String>) = sqlx::query_as(
        "SELECT current_user::text, nullif(current_setting('request.jwt.claims', true), '')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_ne!(
        rol, "authenticated",
        "el rol no debe persistir fuera de la transacción"
    );
    assert_eq!(
        claims, None,
        "los claims no deben persistir fuera de la transacción"
    );

    // El siguiente usuario solo ve lo suyo, aunque use la misma conexión.
    let mut tx = como(&pool, SIN_SERVICIOS).await;
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM servicios")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(n, 0);
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(migrations = false, fixtures("supabase_subset", "rls_test", "datos"))]
async fn verificar_al_arrancar(pool: db::PgPool) {
    assert_eq!(db::rls::verificar(&pool).await, Ok(()));
}

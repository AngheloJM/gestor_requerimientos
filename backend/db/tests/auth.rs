//! Credenciales y sesiones (migración 0002 real vía fixture `autenticacion`).

mod comun;

use chrono::{Duration, Utc};
use comun::*;
use db::auth::*;

fn nueva(hash: &[u8]) -> NuevaSesion<'_> {
    NuevaSesion {
        usuario_id: uid(RRHH),
        token_hash: hash,
        expira_en: Utc::now() + Duration::days(7),
        user_agent: Some("test"),
        ip: Some("127.0.0.1".parse().unwrap()),
    }
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(
    migrations = false,
    fixtures("supabase_subset", "rls_test", "datos", "autenticacion")
)]
async fn credenciales_email_insensible_a_mayusculas(pool: db::PgPool) {
    guardar_credencial(&pool, uid(RRHH), "RRHH@Ejemplo.test", "$argon2id$hash")
        .await
        .unwrap();
    let c = credencial_por_email(&pool, "  rrhh@ejemplo.TEST ")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(c.usuario_id, uid(RRHH));
    assert!(c.activo);
    assert!(
        credencial_por_email(&pool, "otro@ejemplo.test")
            .await
            .unwrap()
            .is_none()
    );
    // Email único.
    assert!(
        guardar_credencial(&pool, uid(COSTOS), "rrhh@ejemplo.test", "x")
            .await
            .is_err()
    );
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(
    migrations = false,
    fixtures("supabase_subset", "rls_test", "datos", "autenticacion")
)]
async fn rotacion_y_deteccion_de_reuso(pool: db::PgPool) {
    crear_sesion(&pool, &nueva(b"t1")).await.unwrap();

    assert_eq!(
        rotar_sesion(&pool, b"t1", &nueva(b"t2")).await.unwrap(),
        ResultadoRotacion::Rotada {
            usuario_id: uid(RRHH)
        }
    );
    assert_eq!(
        rotar_sesion(&pool, b"t2", &nueva(b"t3")).await.unwrap(),
        ResultadoRotacion::Rotada {
            usuario_id: uid(RRHH)
        }
    );
    // Reusar t1 (ya rotado) → se revocan TODAS, incluida t3 vigente.
    assert_eq!(
        rotar_sesion(&pool, b"t1", &nueva(b"t4")).await.unwrap(),
        ResultadoRotacion::Reutilizada {
            usuario_id: uid(RRHH)
        }
    );
    assert_eq!(
        rotar_sesion(&pool, b"t3", &nueva(b"t5")).await.unwrap(),
        ResultadoRotacion::Reutilizada {
            usuario_id: uid(RRHH)
        }
    );
    assert_eq!(
        rotar_sesion(&pool, b"desconocido", &nueva(b"t6"))
            .await
            .unwrap(),
        ResultadoRotacion::Invalida
    );
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(
    migrations = false,
    fixtures("supabase_subset", "rls_test", "datos", "autenticacion")
)]
async fn expirada_logout_e_inactivo(pool: db::PgPool) {
    let mut vencida = nueva(b"v1");
    vencida.expira_en = Utc::now() - Duration::seconds(1);
    crear_sesion(&pool, &vencida).await.unwrap();
    assert_eq!(
        rotar_sesion(&pool, b"v1", &nueva(b"v2")).await.unwrap(),
        ResultadoRotacion::Invalida
    );

    crear_sesion(&pool, &nueva(b"l1")).await.unwrap();
    revocar_sesion(&pool, b"l1").await.unwrap();
    assert!(matches!(
        rotar_sesion(&pool, b"l1", &nueva(b"l2")).await.unwrap(),
        ResultadoRotacion::Reutilizada { .. }
    ));

    crear_sesion(&pool, &nueva(b"i1")).await.unwrap();
    sqlx::query("UPDATE usuarios SET activo = false WHERE id = $1")
        .bind(uid(RRHH))
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        rotar_sesion(&pool, b"i1", &nueva(b"i2")).await.unwrap(),
        ResultadoRotacion::Invalida
    );
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(
    migrations = false,
    fixtures("supabase_subset", "rls_test", "datos", "autenticacion")
)]
async fn tablas_no_accesibles_para_authenticated(pool: db::PgPool) {
    let mut tx = como(&pool, RRHH).await;
    let err = sqlx::query("SELECT count(*) FROM credenciales")
        .fetch_one(&mut *tx)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("permission denied"), "{err}");
}

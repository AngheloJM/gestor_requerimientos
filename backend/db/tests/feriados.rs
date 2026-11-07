mod comun;

use chrono::NaiveDate;
use comun::*;
use db::feriados;

const CALL_CENTER: &str = "00000000-0000-0000-0000-000000000052";
const OPERADOR: &str = "00000000-0000-0000-0000-000000000062";
const TECNICO: &str = "00000000-0000-0000-0000-000000000061";

fn f(m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, m, d).unwrap()
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(
    migrations = false,
    fixtures(
        "supabase_subset",
        "rls_test",
        "datos",
        "p2_edicion",
        "p2_matriz",
        "revision_matriz",
        "feriados"
    )
)]
async fn feriados_los_lee_cualquiera_y_los_escribe_rrhh(pool: db::PgPool) {
    let mut tx = como(&pool, RRHH).await;
    feriados::crear(&mut tx, f(11, 9), "Feriado de prueba")
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let mut tx = como(&pool, RRHH).await;
    let dup = feriados::crear(&mut tx, f(11, 9), "Otro")
        .await
        .unwrap_err();
    assert!(dup.es_duplicado());
    drop(tx);

    let mut tx = como(&pool, ESTANDAR).await;
    let vistos = feriados::entre(&mut tx, f(11, 1), f(12, 1)).await.unwrap();
    assert!(vistos.contains(&f(11, 9)));
    assert_eq!(feriados::del_anio(&mut tx, 2026).await.unwrap().len(), 1);
    assert!(
        feriados::crear(&mut tx, f(12, 25), "Navidad")
            .await
            .is_err()
    );
    drop(tx);

    let mut tx = como(&pool, ESTANDAR).await;
    let id = feriados::del_anio(&mut tx, 2026).await.unwrap()[0].id;
    assert_eq!(feriados::eliminar(&mut tx, id).await.unwrap(), 0);
    drop(tx);

    let mut tx = como(&pool, RRHH).await;
    assert_eq!(feriados::eliminar(&mut tx, id).await.unwrap(), 1);
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(
    migrations = false,
    fixtures(
        "supabase_subset",
        "rls_test",
        "datos",
        "p2_edicion",
        "p2_matriz",
        "revision_matriz",
        "feriados"
    )
)]
async fn tiempos_del_cargo_con_respaldo_del_servicio(pool: db::PgPool) {
    let mut tx = como(&pool, RRHH).await;
    sqlx::query(
        "UPDATE servicio_cargos SET tiempo_capacitacion_dias = 10,
                tiempo_capacitacion_ajustada_dias = 15
          WHERE servicio_id = $1 AND cargo_id = $2",
    )
    .bind(uid(CALL_CENTER))
    .bind(uid(OPERADOR))
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query("UPDATE servicios SET tiempo_capacitacion_dias = 7 WHERE id = $1")
        .bind(uid(CALL_CENTER))
        .execute(&mut *tx)
        .await
        .unwrap();

    let t = feriados::tiempos_capacitacion(&mut tx, uid(CALL_CENTER), uid(OPERADOR))
        .await
        .unwrap();
    assert_eq!(
        (t.capacitacion_cargo, t.ajustada_cargo, t.general_servicio),
        (Some(10), Some(15), Some(7))
    );
    // Cargo que no está en el servicio: solo el general.
    let t = feriados::tiempos_capacitacion(&mut tx, uid(CALL_CENTER), uid(TECNICO))
        .await
        .unwrap();
    assert_eq!(
        (t.capacitacion_cargo, t.ajustada_cargo, t.general_servicio),
        (None, None, Some(7))
    );
}

mod comun;

use chrono::NaiveDate;
use comun::*;
use db::parametros::{SALARIO_MINIMO_NACIONAL, historial, registrar, vigentes};
use rust_decimal_macros::dec;

fn f(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(
    migrations = false,
    fixtures("supabase_subset", "rls_test", "datos", "p2_edicion", "p2_parametros")
)]
async fn vigente_segun_fecha_y_alta_solo_rrhh(pool: db::PgPool) {
    let mut tx = como(&pool, ESTANDAR).await;
    let h = historial(&mut tx).await.unwrap();
    assert_eq!(h.len(), 3, "todos leen");
    let smn = |fecha| {
        vigentes(&h, fecha)
            .into_iter()
            .find(|v| v.parametro == SALARIO_MINIMO_NACIONAL)
            .map(|v| v.valor)
    };
    assert_eq!(smn(f(2026, 10, 27)), Some(dec!(3300)));
    assert_eq!(smn(f(2026, 11, 3)), Some(dec!(3600)));
    assert_eq!(smn(f(2026, 1, 1)), None, "antes de la primera vigencia");

    // Estándar no puede registrar.
    let e = registrar(&mut tx, SALARIO_MINIMO_NACIONAL, dec!(4000), f(2027, 1, 1))
        .await
        .unwrap_err();
    assert!(e.es_permiso_denegado());
    drop(tx);

    // RR.HH. agrega un valor futuro; no pisa los anteriores; misma fecha → duplicado.
    let mut tx = como(&pool, RRHH).await;
    registrar(&mut tx, SALARIO_MINIMO_NACIONAL, dec!(4000), f(2027, 1, 1))
        .await
        .unwrap();
    let h = historial(&mut tx).await.unwrap();
    assert_eq!(h.len(), 4);
    assert_eq!(
        h.iter().find(|v| v.valor == dec!(4000)).unwrap().creado_por,
        Some(uid(RRHH))
    );
    let e = registrar(&mut tx, SALARIO_MINIMO_NACIONAL, dec!(4100), f(2027, 1, 1))
        .await
        .unwrap_err();
    assert!(e.es_duplicado());
}

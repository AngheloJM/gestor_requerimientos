mod comun;

use comun::*;
use db::almanaque::listar_movimientos;
use domain::{Almanaque, LineaAlmanaque, Periodo};

async fn almanaque(pool: &db::PgPool, u: &str) -> Almanaque {
    let periodo = Periodo::new(2026, 11).unwrap();
    let mut tx = como(pool, u).await;
    let movs = listar_movimientos(&mut tx, periodo).await.unwrap();
    Almanaque::construir(periodo, &movs)
}

fn r(l: &[LineaAlmanaque]) -> Vec<(String, String, u64, u32)> {
    l.iter()
        .map(|x| {
            (
                x.servicio_nombre.clone().unwrap_or("-".into()),
                x.cargo_nombre.clone().unwrap_or_default(),
                x.cantidad,
                x.registros_sin_cantidad,
            )
        })
        .collect()
}

fn l(s: &str, c: &str, n: u64, sin: u32) -> (String, String, u64, u32) {
    (s.into(), c.into(), n, sin)
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(migrations = false, fixtures("supabase_subset", "rls_test", "datos"))]
async fn cuatro_categorias_desde_requerimientos_y_entregas(pool: db::PgPool) {
    let a = almanaque(&pool, RRHH).await;
    let d = |n: usize| &a.dias[n - 1];

    // Entradas: Operador 3+2 sumados; Agente con cantidad NULL contado aparte;
    // el requerimiento del servicio inactivo (9) no aparece.
    assert_eq!(
        r(&d(3).entradas),
        [
            l("Call center", "Agente", 0, 1),
            l("Call center", "Operador", 5, 0)
        ]
    );
    // Requerimiento sin servicio: se incluye con servicio vacío.
    assert_eq!(r(&d(10).entradas), [l("-", "Analista", 1, 0)]);
    assert_eq!(
        r(&d(20).salidas),
        [
            l("Call center", "Operador", 3, 0),
            l("Soporte", "Técnico", 4, 0)
        ]
    );
    // Entregas (sin corrige_entrega_id); la del servicio inactivo no aparece.
    assert_eq!(
        r(&d(5).entregas),
        [
            l("Call center", "Operador", 3, 0),
            l("Soporte", "Técnico", 4, 0)
        ]
    );
    // Rectificación (con corrige_entrega_id), separada de las entregas.
    assert_eq!(
        r(&d(6).rectificaciones),
        [l("Call center", "Operador", 2, 0)]
    );
    assert!(d(6).entregas.is_empty());
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(migrations = false, fixtures("supabase_subset", "rls_test", "datos"))]
async fn visibilidad_la_decide_rls(pool: db::PgPool) {
    let a = almanaque(&pool, COSTOS).await;
    assert_eq!(r(&a.dias[19].salidas), [l("Call center", "Operador", 3, 0)]);
    assert_eq!(r(&a.dias[4].entregas), [l("Call center", "Operador", 3, 0)]);
    let a = almanaque(&pool, SIN_SERVICIOS).await;
    assert!(
        a.dias
            .iter()
            .all(|d| d.entradas.is_empty() && d.entregas.is_empty())
    );
}

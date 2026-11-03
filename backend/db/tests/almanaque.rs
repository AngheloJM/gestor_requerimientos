mod comun;

use comun::*;
use db::almanaque::listar_movimientos;
use domain::{Almanaque, LineaAlmanaque, Periodo};

// Usa la copia textual de `leer_eventos_almanaque` (fixture p2_almanaque).

async fn almanaque(pool: &db::PgPool, u: &str) -> Result<Almanaque, db::DbError> {
    let periodo = Periodo::new(2026, 11).unwrap();
    let mut tx = como(pool, u).await;
    let movs = listar_movimientos(&mut tx, periodo).await?;
    Ok(Almanaque::construir(periodo, &movs))
}

type Fila = (
    String,
    String,
    String,
    i64,
    u32,
    Option<String>,
    Vec<String>,
);

fn r(l: &[LineaAlmanaque]) -> Vec<Fila> {
    l.iter()
        .map(|x| {
            (
                x.cliente.clone().unwrap_or("-".into()),
                x.servicio.clone().unwrap_or("-".into()),
                x.cargo.clone().unwrap_or_default(),
                x.cantidad,
                x.registros_sin_cantidad,
                x.estado_entrega.clone(),
                x.folios.clone(),
            )
        })
        .collect()
}

fn l(cli: &str, s: &str, c: &str, n: i64, sin: u32, estado: Option<&str>, folios: &[&str]) -> Fila {
    (
        cli.into(),
        s.into(),
        c.into(),
        n,
        sin,
        estado.map(Into::into),
        folios.iter().map(|f| (*f).to_owned()).collect(),
    )
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(
    migrations = false,
    fixtures("supabase_subset", "rls_test", "datos", "p2_almanaque")
)]
async fn cuatro_categorias_con_cliente_folio_y_estado(pool: db::PgPool) {
    let a = almanaque(&pool, RRHH).await.unwrap();
    let d = |n: usize| &a.dias[n - 1];

    // Entradas del 3: Operador 3+2 sumados; Agente con cantidad NULL contado aparte.
    assert_eq!(
        r(&d(3).entradas),
        [
            l(
                "Cliente Alfa",
                "Call center",
                "Agente",
                0,
                1,
                None,
                &["REQ-000003"]
            ),
            l(
                "Cliente Alfa",
                "Call center",
                "Operador",
                5,
                0,
                None,
                &["REQ-000001", "REQ-000002"]
            ),
        ]
    );
    // El del servicio inactivo (Legado, 1/11) no aparece; el sin servicio sí.
    assert!(d(1).entradas.is_empty(), "servicio inactivo excluido");
    assert_eq!(
        r(&d(10).entradas),
        [l("-", "-", "Analista", 1, 0, None, &["REQ-000006"])]
    );
    assert_eq!(
        r(&d(20).salidas),
        [
            l(
                "Cliente Alfa",
                "Call center",
                "Operador",
                3,
                0,
                None,
                &["REQ-000001"]
            ),
            l(
                "Cliente Beta",
                "Soporte",
                "Técnico",
                4,
                0,
                None,
                &["REQ-000004"]
            ),
        ]
    );
    // Entregas con el estado de su decisión; la del servicio inactivo no aparece.
    assert_eq!(
        r(&d(5).entregas),
        [
            l(
                "Cliente Alfa",
                "Call center",
                "Operador",
                3,
                0,
                Some("aceptada"),
                &["REQ-000001"]
            ),
            l(
                "Cliente Beta",
                "Soporte",
                "Técnico",
                4,
                0,
                Some("pendiente"),
                &["REQ-000004"]
            ),
        ]
    );
    assert_eq!(
        r(&d(6).rectificaciones),
        [l(
            "Cliente Alfa",
            "Call center",
            "Operador",
            2,
            0,
            Some("pendiente"),
            &["REQ-000001"]
        )]
    );
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(
    migrations = false,
    fixtures("supabase_subset", "rls_test", "datos", "p2_almanaque")
)]
async fn acceso_lo_decide_la_funcion_de_la_base(pool: db::PgPool) {
    // Costos: solo sus servicios asignados (Call center, Campo, Instalaciones).
    let a = almanaque(&pool, COSTOS).await.unwrap();
    let servicios: Vec<_> = a
        .dias
        .iter()
        .flat_map(|d| d.entradas.iter().chain(&d.salidas).chain(&d.entregas))
        .filter_map(|x| x.servicio.as_deref())
        .collect();
    assert!(!servicios.is_empty() && servicios.iter().all(|s| *s == "Call center"));

    // Estándar sin servicios asignados: nada.
    let a = almanaque(&pool, SIN_SERVICIOS).await.unwrap();
    assert!(
        a.dias
            .iter()
            .all(|d| d.entradas.is_empty() && d.entregas.is_empty())
    );

    // Externo: la función lo rechaza (42501 → 403 en la API).
    let e = almanaque(&pool, "00000000-0000-0000-0000-aaaa00000005")
        .await
        .unwrap_err();
    assert!(e.es_permiso_denegado());
}

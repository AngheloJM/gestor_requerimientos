mod comun;

use comun::*;
use db::matriz::listar_matriz;
use domain::{CentroCostoId, ClienteId, FilaMatriz, FiltrosMatriz, ProyectoId, ServicioId};
use rust_decimal_macros::dec;

async fn matriz(pool: &db::PgPool, u: &str, f: FiltrosMatriz) -> Vec<FilaMatriz> {
    let mut tx = como(pool, u).await;
    listar_matriz(&mut tx, f).await.unwrap()
}

fn etiquetas(v: &[FilaMatriz]) -> Vec<String> {
    v.iter()
        .map(|f| {
            format!(
                "{}|{}|{}|{}",
                f.centro_costo.as_ref().map_or("-", |c| c.codigo.as_str()),
                f.cliente.as_ref().map_or("-", |c| c.nombre.as_str()),
                f.servicio.nombre,
                f.cargo.nombre
            )
        })
        .collect()
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(migrations = false, fixtures("supabase_subset", "rls_test", "datos"))]
async fn orden_centro_costo_cliente_servicio_cargo_y_solo_activos(pool: db::PgPool) {
    assert_eq!(
        etiquetas(&matriz(&pool, RRHH, FiltrosMatriz::default()).await),
        [
            "CC-01|Cliente Alfa|Call center|Agente",
            "CC-01|Cliente Alfa|Call center|Operador",
            "CC-01|Cliente Beta|Soporte|Técnico",
            "CC-02|Cliente Alfa|Campo|Operador",
            "CC-02|Cliente Alfa|Campo|Supervisor",
            "-|Cliente Beta|Mesa de ayuda|Analista", // sin centro de costo: al final
        ] // "Legado" (inactivo) no aparece.
    );
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(migrations = false, fixtures("supabase_subset", "rls_test", "datos"))]
async fn cargo_es_catalogo_compartido_y_composicion_por_servicio(pool: db::PgPool) {
    let filas = matriz(&pool, RRHH, FiltrosMatriz::default()).await;
    let operadores: Vec<_> = filas
        .iter()
        .filter(|f| f.cargo.nombre == "Operador")
        .collect();
    assert_eq!(operadores.len(), 2);
    assert_eq!(
        operadores[0].cargo.id, operadores[1].cargo.id,
        "mismo cargo del catálogo"
    );
    assert_ne!(operadores[0].composicion.id, operadores[1].composicion.id);
    assert_eq!(operadores[0].composicion.haber_basico_diurno, dec!(3000.00));
    assert_eq!(operadores[1].composicion.haber_basico_diurno, dec!(3100.00));
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(migrations = false, fixtures("supabase_subset", "rls_test", "datos"))]
async fn metricas_y_umbrales(pool: db::PgPool) {
    let filas = matriz(&pool, RRHH, FiltrosMatriz::default()).await;
    let op = filas
        .iter()
        .find(|f| f.servicio.nombre == "Call center" && f.cargo.nombre == "Operador")
        .unwrap();
    let c = &op.composicion;
    assert_eq!(c.total_haberes(), dec!(3500.00));
    assert_eq!(
        c.total_ganado(),
        dec!(3800.00),
        "haberes + bonos AHT 200 y Calidad 100"
    );
    let nombres: Vec<_> = c.metricas.iter().map(|m| m.nombre.as_str()).collect();
    assert_eq!(nombres, ["AHT", "Calidad"]);
    let aht = &c.metricas[0];
    assert_eq!((aht.peso_pct, aht.monto_bono), (dec!(60), dec!(200.00)));
    let ordenes: Vec<_> = aht.umbrales.iter().map(|u| u.orden).collect();
    assert_eq!(ordenes, [1, 2]);
    assert_eq!(aht.umbral_para(dec!(85)).map(|u| u.orden), Some(2));
    assert_eq!(
        op.proyecto.as_ref().map(|p| p.nombre.as_str()),
        Some("Proyecto Norte")
    );
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(migrations = false, fixtures("supabase_subset", "rls_test", "datos"))]
async fn filtros(pool: db::PgPool) {
    let cargos = |v: Vec<FilaMatriz>| -> Vec<String> {
        v.into_iter()
            .map(|f| format!("{}/{}", f.servicio.nombre, f.cargo.nombre))
            .collect()
    };
    let f = FiltrosMatriz {
        centro_costo_id: Some(CentroCostoId(uid("00000000-0000-0000-0000-0000000000c2"))),
        ..Default::default()
    };
    assert_eq!(
        cargos(matriz(&pool, RRHH, f).await),
        ["Campo/Operador", "Campo/Supervisor"]
    );
    let f = FiltrosMatriz {
        cliente_id: Some(ClienteId(uid("00000000-0000-0000-0000-0000000000b1"))),
        ..Default::default()
    };
    assert_eq!(
        cargos(matriz(&pool, RRHH, f).await),
        ["Soporte/Técnico", "Mesa de ayuda/Analista"]
    );
    let f = FiltrosMatriz {
        proyecto_id: Some(ProyectoId(uid("00000000-0000-0000-0000-0000000000f0"))),
        ..Default::default()
    };
    assert_eq!(
        cargos(matriz(&pool, RRHH, f).await),
        ["Call center/Agente", "Call center/Operador"]
    );
    let f = FiltrosMatriz {
        servicio_id: Some(ServicioId(uid("00000000-0000-0000-0000-000000000056"))),
        ..Default::default()
    };
    assert!(matriz(&pool, RRHH, f).await.is_empty(), "servicio inactivo");
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(migrations = false, fixtures("supabase_subset", "rls_test", "datos"))]
async fn visibilidad_la_decide_rls(pool: db::PgPool) {
    assert_eq!(
        etiquetas(&matriz(&pool, COSTOS, FiltrosMatriz::default()).await),
        [
            "CC-01|Cliente Alfa|Call center|Agente",
            "CC-01|Cliente Alfa|Call center|Operador",
            "CC-02|Cliente Alfa|Campo|Operador",
            "CC-02|Cliente Alfa|Campo|Supervisor",
        ]
    );
    assert!(
        matriz(&pool, SIN_SERVICIOS, FiltrosMatriz::default())
            .await
            .is_empty()
    );
}

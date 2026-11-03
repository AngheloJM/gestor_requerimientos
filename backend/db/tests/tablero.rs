mod comun;

use chrono::NaiveDate;
use comun::*;
use db::tablero::{estado_servicios, solicitudes_cambio};
use domain::{Completitud, Indicadores, Kanban};

fn hoy() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 2).unwrap()
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(migrations = false, fixtures("supabase_subset", "rls_test", "datos"))]
async fn indicadores_por_completitud(pool: db::PgPool) {
    let mut tx = como(&pool, RRHH).await;
    let c = Completitud::construir(&estado_servicios(&mut tx).await.unwrap(), hoy());
    assert_eq!(
        c.indicadores,
        Indicadores {
            servicios_total: 5, // activos
            servicios_configurados: 2,
            servicios_incompletos: 3,
            areas_sin_completar: 2,
        }
    );
    let detalle: Vec<_> = c
        .incompletos
        .iter()
        .map(|i| {
            (
                i.servicio_nombre.as_str(),
                i.tareas.iter().map(|t| t.codigo()).collect::<Vec<_>>(),
            )
        })
        .collect();
    assert_eq!(
        detalle,
        [
            (
                "Instalaciones",
                vec!["verificar_vigencia_contrato", "definir_cargos"]
            ),
            ("Mesa de ayuda", vec!["asignar_contrato_comercial"]),
            ("Soporte", vec!["contrato_no_vigente"]),
        ]
    );
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(migrations = false, fixtures("supabase_subset", "rls_test", "datos"))]
async fn kanban_desde_solicitudes_cambio_con_visibilidad_rls(pool: db::PgPool) {
    async fn ids(pool: &db::PgPool, u: &str) -> Vec<String> {
        let mut tx = como(pool, u).await;
        let k = Kanban::construir(solicitudes_cambio(&mut tx).await.unwrap());
        assert!(k.comercial.is_empty() && k.operaciones.is_empty());
        k.rrhh
            .iter()
            .map(|s| s.id.to_string()[34..].to_string())
            .collect()
    }
    // RR.HH. ve todas las de servicios activos (la 33 es de un servicio inactivo).
    assert_eq!(ids(&pool, RRHH).await, ["31", "32", "34"]);
    // Costos: solo campo = 'centro_costo'.
    assert_eq!(ids(&pool, COSTOS).await, ["31"]);
    // Solicitante sin servicios: ve las suyas aunque no vea el servicio. Incluye la 33
    // (servicio inactivo): como RLS le oculta el servicio, no se puede saber si está activo.
    // TODO(negocio/DB): ver docs/PENDIENTES.md ("Kanban y servicios no visibles").
    assert_eq!(ids(&pool, SIN_SERVICIOS).await, ["31", "32", "33"]);
}

mod comun;

use comun::*;
use db::solicitudes::{CerrarError, Cierre, NuevaSolicitud, cerrar, crear};
use domain::{CargoId, ServicioId};

const SOL_CC: &str = "00000000-0000-0000-0000-000000000031"; // centro_costo, de SIN_SERVICIOS
const SOL_SUELDO: &str = "00000000-0000-0000-0000-000000000032"; // sueldo_basico, de SIN_SERVICIOS
const SOL_SOPORTE: &str = "00000000-0000-0000-0000-000000000034"; // modalidad, de ESTANDAR

async fn estado(pool: &db::PgPool, id: &str) -> (String, Option<uuid::Uuid>, Option<String>, bool) {
    sqlx::query_as(
        "SELECT estado, responsable_id, comentario_resolucion, cerrada_at IS NOT NULL
         FROM solicitudes_cambio WHERE id = $1",
    )
    .bind(uid(id))
    .fetch_one(pool)
    .await
    .unwrap()
}

fn regla(e: CerrarError) -> String {
    match e {
        CerrarError::Db(e) => e.regla_de_negocio().expect("regla").to_owned(),
        otro => panic!("esperaba regla de negocio, llegó {otro:?}"),
    }
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(
    migrations = false,
    fixtures("supabase_subset", "rls_test", "datos", "p2_edicion", "p2_solicitudes")
)]
async fn resolver_rechazar_y_cancelar_segun_reglas(pool: db::PgPool) {
    // RR.HH. resuelve con comentario: la base fija responsable y cierre.
    let mut tx = como(&pool, RRHH).await;
    cerrar(
        &mut tx,
        uid(SOL_SUELDO),
        Cierre::Resolver,
        Some("  Sueldo corregido "),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(
        estado(&pool, SOL_SUELDO).await,
        (
            "resuelta".into(),
            Some(uid(RRHH)),
            Some("Sueldo corregido".into()),
            true
        )
    );

    // Ya cerrada: no se puede volver a cerrar (RLS solo deja tocar pendientes).
    let mut tx = como(&pool, RRHH).await;
    assert!(matches!(
        cerrar(&mut tx, uid(SOL_SUELDO), Cierre::Rechazar, Some("otra vez")).await,
        Err(CerrarError::NoDisponible)
    ));
    drop(tx);

    // Rechazar sin comentario: lo rechaza la base.
    let mut tx = como(&pool, COSTOS).await;
    assert_eq!(
        regla(
            cerrar(&mut tx, uid(SOL_CC), Cierre::Rechazar, None)
                .await
                .unwrap_err()
        ),
        "El comentario de resolución es obligatorio"
    );
    drop(tx);
    // Costos sí puede cerrar las de centro de costo.
    let mut tx = como(&pool, COSTOS).await;
    cerrar(
        &mut tx,
        uid(SOL_CC),
        Cierre::Rechazar,
        Some("Centro correcto"),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(estado(&pool, SOL_CC).await.0, "rechazada");

    // Cancelar: solo quien la pidió.
    let mut tx = como(&pool, RRHH).await;
    assert_eq!(
        regla(
            cerrar(&mut tx, uid(SOL_SOPORTE), Cierre::Cancelar, None)
                .await
                .unwrap_err()
        ),
        "Solo quien pidió el cambio puede cancelarlo"
    );
    drop(tx);
    let mut tx = como(&pool, ESTANDAR).await;
    cerrar(&mut tx, uid(SOL_SOPORTE), Cierre::Cancelar, None)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(
        estado(&pool, SOL_SOPORTE).await,
        ("cancelada".into(), None, None, true)
    );
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(
    migrations = false,
    fixtures("supabase_subset", "rls_test", "datos", "p2_edicion", "p2_solicitudes")
)]
async fn crear_solicitud(pool: db::PgPool) {
    let soporte = ServicioId(uid("00000000-0000-0000-0000-000000000051"));
    let mut tx = como(&pool, ESTANDAR).await;
    let id = crear(
        &mut tx,
        &NuevaSolicitud {
            servicio_id: soporte,
            cargo_id: Some(CargoId(uid("00000000-0000-0000-0000-000000000061"))),
            campo: "sueldo_basico",
            motivo: "  El sueldo cambió en octubre ",
            descripcion: Some(""),
        },
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let (estado_, motivo, desc): (String, String, Option<String>) =
        sqlx::query_as("SELECT estado, motivo, descripcion FROM solicitudes_cambio WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        (estado_.as_str(), motivo.as_str(), desc),
        ("pendiente", "El sueldo cambió en octubre", None)
    );

    // Sin el servicio asignado: RLS lo impide.
    let mut tx = como(&pool, ESTANDAR).await;
    let e = crear(
        &mut tx,
        &NuevaSolicitud {
            servicio_id: ServicioId(uid("00000000-0000-0000-0000-000000000052")),
            cargo_id: None,
            campo: "modalidad",
            motivo: "No es mío",
            descripcion: None,
        },
    )
    .await
    .unwrap_err();
    assert!(e.es_permiso_denegado());
}

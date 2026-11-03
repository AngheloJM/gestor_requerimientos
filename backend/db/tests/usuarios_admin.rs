mod comun;

use comun::*;
use db::usuarios::{DatosUsuario, EditarUsuarioError, actualizar_admin, listar_admin};

const SOPORTE: &str = "00000000-0000-0000-0000-000000000051";
const CALL_CENTER: &str = "00000000-0000-0000-0000-000000000052";
const TECNICO: &str = "00000000-0000-0000-0000-000000000061";

fn datos(servicios: Vec<uuid::Uuid>) -> DatosUsuario {
    DatosUsuario {
        cargo_id: Some(uid(TECNICO)),
        division: Some("  Operaciones ".into()),
        region: Some("".into()),
        inmediato_superior_id: Some(uid(RRHH)),
        rol: "estandar".into(),
        activo: true,
        servicios,
    }
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(
    migrations = false,
    fixtures("supabase_subset", "rls_test", "datos", "p2_edicion", "p2_usuarios")
)]
async fn rrhh_edita_datos_y_servicios(pool: db::PgPool) {
    let mut tx = como(&pool, RRHH).await;
    let antes = listar_admin(&mut tx).await.unwrap();
    assert_eq!(antes.len(), 5);
    let est = antes.iter().find(|u| u.id == uid(ESTANDAR)).unwrap();
    assert_eq!(
        est.servicios
            .iter()
            .map(|s| s.1.as_str())
            .collect::<Vec<_>>(),
        ["Soporte"]
    );

    // Cambia datos y servicios: quita Soporte, agrega Call center.
    actualizar_admin(&mut tx, uid(ESTANDAR), &datos(vec![uid(CALL_CENTER)]))
        .await
        .unwrap();
    let u = listar_admin(&mut tx)
        .await
        .unwrap()
        .into_iter()
        .find(|u| u.id == uid(ESTANDAR))
        .unwrap();
    assert_eq!(u.cargo_nombre.as_deref(), Some("Técnico"));
    assert_eq!(
        (u.division.as_deref(), u.region.as_deref()),
        (Some("Operaciones"), None)
    );
    assert_eq!(u.superior_nombre.as_deref(), Some("RRHH"));
    assert_eq!(
        u.servicios.iter().map(|s| s.1.as_str()).collect::<Vec<_>>(),
        ["Call center"]
    );

    // Desactivar y cambiar rol.
    let mut d = datos(vec![uid(SOPORTE)]);
    d.activo = false;
    d.rol = "encargado_costos".into();
    actualizar_admin(&mut tx, uid(ESTANDAR), &d).await.unwrap();
    let u = listar_admin(&mut tx)
        .await
        .unwrap()
        .into_iter()
        .find(|u| u.id == uid(ESTANDAR))
        .unwrap();
    assert_eq!((u.rol.as_str(), u.activo), ("encargado_costos", false));
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(
    migrations = false,
    fixtures("supabase_subset", "rls_test", "datos", "p2_edicion", "p2_usuarios")
)]
async fn solo_rrhh_edita_y_la_base_valida_el_rol(pool: db::PgPool) {
    // Estándar ve la lista (política SELECT = true) pero no puede editar.
    let mut tx = como(&pool, ESTANDAR).await;
    assert_eq!(listar_admin(&mut tx).await.unwrap().len(), 5);
    assert!(matches!(
        actualizar_admin(&mut tx, uid(SIN_SERVICIOS), &datos(vec![])).await,
        Err(EditarUsuarioError::NoEncontrado)
    ));
    drop(tx);

    // Rol fuera del CHECK: lo rechaza la base aunque la API ya lo valide.
    let mut tx = como(&pool, RRHH).await;
    let mut d = datos(vec![]);
    d.rol = "gerente".into();
    assert!(matches!(
        actualizar_admin(&mut tx, uid(ESTANDAR), &d).await,
        Err(EditarUsuarioError::Db(_))
    ));
}

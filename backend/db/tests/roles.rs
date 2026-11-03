mod comun;

use comun::*;
use db::roles::{RolError, actualizar, catalogo, crear, eliminar, listar, obtener};
use domain::{DatosRol, PermisoAsignado};

fn p(clave: &str, alcance: &str) -> PermisoAsignado {
    PermisoAsignado {
        clave: clave.into(),
        alcance: alcance.into(),
    }
}

fn datos(nombre: &str, permisos: Vec<PermisoAsignado>) -> DatosRol {
    DatosRol {
        nombre: nombre.into(),
        descripcion: Some("  Aprueba y revisa sueldos ".into()),
        activo: true,
        permisos,
    }
}

async fn historial(
    pool: &db::PgPool,
    rol: uuid::Uuid,
) -> Vec<(String, String, Option<String>, Option<String>)> {
    sqlx::query_as(
        "SELECT permiso_clave, cambio, alcance_anterior, alcance_nuevo FROM rol_permisos_historial
         WHERE rol_id = $1 ORDER BY created_at, permiso_clave, cambio",
    )
    .bind(rol)
    .fetch_all(pool)
    .await
    .unwrap()
}

fn regla(e: RolError) -> String {
    match e {
        RolError::Db(e) => e.regla_de_negocio().expect("regla de negocio").to_owned(),
        otro => panic!("esperaba regla de negocio, llegó {otro:?}"),
    }
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(
    migrations = false,
    fixtures("supabase_subset", "rls_test", "datos", "p2_edicion", "roles")
)]
async fn catalogo_y_roles_de_sistema(pool: db::PgPool) {
    let mut tx = como(&pool, ESTANDAR).await;
    let c = catalogo(&mut tx).await.unwrap();
    assert!(c.len() >= 70, "catálogo granular");
    assert!(c.iter().any(|p| p.clave == "sueldos.ver_montos"));
    let roles = listar(&mut tx).await.unwrap();
    let claves: Vec<_> = roles.iter().map(|r| r.clave.as_str()).collect();
    assert_eq!(
        claves,
        ["encargado_costos", "estandar", "externo", "rrhh_admin"],
        "los 4 de usuarios.rol, de sistema"
    );
    let rrhh = roles.iter().find(|r| r.clave == "rrhh_admin").unwrap();
    assert_eq!(
        rrhh.permisos.len(),
        c.len(),
        "rrhh_admin: todos los permisos"
    );
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(
    migrations = false,
    fixtures("supabase_subset", "rls_test", "datos", "p2_edicion", "roles")
)]
async fn crear_editar_y_auditar(pool: db::PgPool) {
    let mut tx = como(&pool, RRHH).await;
    let id = crear(
        &mut tx,
        &datos(
            "Gerente  de Área",
            vec![
                p("requerimientos.ver", "servicios_asignados"),
                p("requerimientos.aprobar", "servicios_asignados"),
                p("sueldos.editar", "todos"),
            ],
        ),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let mut tx = como(&pool, RRHH).await;
    let r = obtener(&mut tx, id).await.unwrap().unwrap();
    assert_eq!(
        (r.clave.as_str(), r.nombre.as_str()),
        ("gerente_de_area", "Gerente de Área")
    );
    assert_eq!(r.descripcion.as_deref(), Some("Aprueba y revisa sueldos"));
    assert!(!r.es_sistema);
    assert_eq!(r.creado_por.map(uuid::Uuid::from), Some(uid(RRHH)));

    // Editar: quita sueldos.editar, cambia alcance de aprobar, agrega sueldos.ver_montos.
    actualizar(
        &mut tx,
        id,
        &datos(
            "Gerente de Área",
            vec![
                p("requerimientos.ver", "servicios_asignados"),
                p("requerimientos.aprobar", "todos"),
                p("sueldos.ver_montos", "propios"),
            ],
        ),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let h = historial(&pool, id).await;
    let cambios: Vec<_> = h
        .iter()
        .map(|(c, k, _, _)| (c.as_str(), k.as_str()))
        .collect();
    for esperado in [
        ("requerimientos.ver", "otorgado"),
        ("requerimientos.aprobar", "otorgado"),
        ("sueldos.editar", "otorgado"),
        ("sueldos.editar", "revocado"),
        ("requerimientos.aprobar", "alcance"),
        ("sueldos.ver_montos", "otorgado"),
    ] {
        assert!(
            cambios.contains(&esperado),
            "falta {esperado:?} en {cambios:?}"
        );
    }
    assert_eq!(
        h.len(),
        6,
        "requerimientos.ver sin cambios no se registra de nuevo"
    );
    let alcance = h.iter().find(|(_, k, _, _)| k == "alcance").unwrap();
    assert_eq!(
        (alcance.2.as_deref(), alcance.3.as_deref()),
        (Some("servicios_asignados"), Some("todos"))
    );

    // Eliminar un rol no de sistema.
    let mut tx = como(&pool, RRHH).await;
    eliminar(&mut tx, id).await.unwrap();
    assert!(obtener(&mut tx, id).await.unwrap().is_none());
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(
    migrations = false,
    fixtures("supabase_subset", "rls_test", "datos", "p2_edicion", "roles")
)]
async fn reglas_y_permisos(pool: db::PgPool) {
    // Nombre duplicado (sin distinguir mayúsculas).
    let mut tx = como(&pool, RRHH).await;
    crear(&mut tx, &datos("Gerente", vec![])).await.unwrap();
    assert!(matches!(
        crear(&mut tx, &datos("gerente", vec![])).await,
        Err(RolError::Duplicado)
    ));
    drop(tx);

    // Roles de sistema: no se editan ni se borran.
    let mut tx = como(&pool, RRHH).await;
    let sistema = listar(&mut tx)
        .await
        .unwrap()
        .into_iter()
        .find(|r| r.clave == "estandar")
        .unwrap();
    assert!(
        regla(
            actualizar(&mut tx, sistema.id, &datos("Estándar", vec![]))
                .await
                .unwrap_err()
        )
        .contains("roles de sistema")
    );
    drop(tx);
    let mut tx = como(&pool, RRHH).await;
    assert!(regla(eliminar(&mut tx, sistema.id).await.unwrap_err()).contains("roles de sistema"));
    drop(tx);

    // Alcance limitado en un permiso que no lo admite (lo rechaza la base).
    let mut tx = como(&pool, RRHH).await;
    let e = crear(
        &mut tx,
        &datos("Auditor", vec![p("clientes.ver", "propios")]),
    )
    .await
    .unwrap_err();
    assert!(regla(e).contains("no admite alcance"));
    drop(tx);

    // Solo RR.HH. administra roles (RLS).
    let mut tx = como(&pool, ESTANDAR).await;
    match crear(&mut tx, &datos("Intruso", vec![])).await {
        Err(RolError::Db(e)) => assert!(e.es_permiso_denegado()),
        otro => panic!("esperaba permiso denegado, llegó {otro:?}"),
    }
}

mod comun;

use chrono::NaiveDate;
use comun::*;
use db::{DbError, configuracion as cfg};
use rust_decimal_macros::dec;

const ALFA: &str = "00000000-0000-0000-0000-0000000000a1";
const BETA: &str = "00000000-0000-0000-0000-0000000000b1";
const CALL_CENTER: &str = "00000000-0000-0000-0000-000000000052";

fn regla(e: DbError) -> String {
    e.regla_de_negocio().expect("regla de negocio").to_owned()
}

fn f(m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, m, d).unwrap()
}

fn servicio(
    nombre: &str,
    cliente: Option<&str>,
    proyecto: Option<uuid::Uuid>,
) -> cfg::DatosServicio {
    cfg::DatosServicio {
        codigo: None,
        nombre: nombre.into(),
        centro_costo_id: None,
        contrato_comercial_id: None,
        cliente_id: cliente.map(uid),
        proyecto_id: proyecto,
        tipo_contrato: Some("Indefinido".into()),
        modalidad: "Presencial".into(),
        tipo_turno: Some("turno".into()),
        dias_trabajo: vec!["Lunes".into(), "Martes".into()],
        departamento_region: None,
        manual_funciones_codigo: None,
        tiempo_capacitacion_dias: Some(10),
        capacitacion_pagada: true,
        tiene_refrigerio: false,
        activo: true,
        tiempo_ojt_dias: None,
        tiempo_capacitacion_ajustada_dias: None,
    }
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
        "revision_matriz"
    )
)]
async fn clientes_proyectos_y_servicios(pool: db::PgPool) {
    let mut tx = como(&pool, RRHH).await;
    cfg::crear_cliente(
        &mut tx,
        &cfg::DatosCliente {
            nombre: "Gamma".into(),
            logo_url: None,
            manual_marca_url: Some("https://x/manual.pdf".into()),
        },
    )
    .await
    .unwrap();
    assert!(
        cfg::crear_cliente(
            &mut tx,
            &cfg::DatosCliente {
                nombre: "Gamma".into(),
                logo_url: None,
                manual_marca_url: None
            }
        )
        .await
        .unwrap_err()
        .es_duplicado()
    );
    drop(tx);

    let mut tx = como(&pool, RRHH).await;
    let gamma = cfg::crear_cliente(
        &mut tx,
        &cfg::DatosCliente {
            nombre: "Gamma".into(),
            logo_url: None,
            manual_marca_url: None,
        },
    )
    .await
    .unwrap();
    let proyecto = cfg::crear_proyecto(
        &mut tx,
        &cfg::DatosProyecto {
            cliente_id: gamma,
            nombre: "  Norte 2 ".into(),
        },
    )
    .await
    .unwrap();
    // Servicio con proyecto de otro cliente: lo rechaza el trigger de la base.
    let e = cfg::crear_servicio(&mut tx, &servicio("Nuevo", Some(ALFA), Some(proyecto)))
        .await
        .unwrap_err();
    assert_eq!(
        regla(e),
        "El proyecto no corresponde al cliente del servicio."
    );
    drop(tx);

    let mut tx = como(&pool, RRHH).await;
    let gamma = cfg::crear_cliente(
        &mut tx,
        &cfg::DatosCliente {
            nombre: "Gamma".into(),
            logo_url: None,
            manual_marca_url: None,
        },
    )
    .await
    .unwrap();
    let proyecto = cfg::crear_proyecto(
        &mut tx,
        &cfg::DatosProyecto {
            cliente_id: gamma,
            nombre: "Norte 2".into(),
        },
    )
    .await
    .unwrap();
    let sid = cfg::crear_servicio(
        &mut tx,
        &servicio("Nuevo", Some(&gamma.to_string()), Some(proyecto)),
    )
    .await
    .unwrap();
    let s = cfg::listar_servicios(&mut tx)
        .await
        .unwrap()
        .into_iter()
        .find(|s| s.id == sid)
        .unwrap();
    assert_eq!(
        (s.cliente_nombre.as_deref(), s.proyecto_nombre.as_deref()),
        (Some("Gamma"), Some("Norte 2"))
    );
    assert_eq!(s.dias_trabajo, ["Lunes", "Martes"]);
    // Cambiar el cliente de un proyecto con servicios: lo rechaza la base.
    let e = cfg::actualizar_proyecto(
        &mut tx,
        proyecto,
        &cfg::DatosProyecto {
            cliente_id: uid(BETA),
            nombre: "Norte 2".into(),
        },
    )
    .await
    .unwrap_err();
    assert_eq!(
        regla(e),
        "No se puede cambiar el cliente de un proyecto con servicios."
    );
    drop(tx);

    // Borrar un cliente en uso → FK; un servicio con cargos → FK.
    let mut tx = como(&pool, RRHH).await;
    assert!(
        cfg::eliminar_cliente(&mut tx, uid(ALFA))
            .await
            .unwrap_err()
            .es_fk()
    );
    drop(tx);
    let mut tx = como(&pool, RRHH).await;
    assert!(
        cfg::eliminar_servicio(&mut tx, uid(CALL_CENTER))
            .await
            .unwrap_err()
            .es_fk()
    );
    drop(tx);

    // Estándar no escribe (RLS).
    let mut tx = como(&pool, ESTANDAR).await;
    assert!(
        cfg::crear_cliente(
            &mut tx,
            &cfg::DatosCliente {
                nombre: "X".into(),
                logo_url: None,
                manual_marca_url: None
            }
        )
        .await
        .unwrap_err()
        .es_permiso_denegado()
    );
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
        "revision_matriz"
    )
)]
async fn contratos_centros_cargos_y_catalogos(pool: db::PgPool) {
    let contrato =
        |fin: Option<NaiveDate>, valor: Option<i32>, unidad: Option<&str>| cfg::DatosContrato {
            codigo: "K-NUEVO".into(),
            nombre: Some("Nuevo".into()),
            cliente_id: None,
            fecha_inicio: Some(f(1, 1)),
            fecha_fin: fin,
            duracion_valor: valor,
            duracion_unidad: unidad.map(Into::into),
            tiempo_renovacion_valor: None,
            tiempo_renovacion_unidad: None,
        };
    let mut tx = como(&pool, RRHH).await;
    assert_eq!(
        regla(
            cfg::crear_contrato(&mut tx, &contrato(Some(f(12, 31)), Some(12), Some("meses")))
                .await
                .unwrap_err()
        ),
        "Elige fecha de fin o duración, no ambas."
    );
    drop(tx);
    let mut tx = como(&pool, RRHH).await;
    assert_eq!(
        regla(
            cfg::crear_contrato(&mut tx, &contrato(None, Some(12), Some("años")))
                .await
                .unwrap_err()
        ),
        "La unidad de duración debe ser meses o días."
    );
    drop(tx);
    let mut tx = como(&pool, RRHH).await;
    let k = cfg::crear_contrato(&mut tx, &contrato(None, Some(12), Some("meses")))
        .await
        .unwrap();
    assert_eq!(
        cfg::actualizar_contrato(&mut tx, k, &contrato(Some(f(6, 30)), None, None))
            .await
            .unwrap(),
        1
    );

    let cc = cfg::crear_centro(
        &mut tx,
        &cfg::DatosCentroCosto {
            codigo: "CC-09".into(),
            nombre: "Nueve".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(cfg::eliminar_centro(&mut tx, cc).await.unwrap(), 1);

    // Cargos: mismo nombre con otro tipo sí; mismo nombre y tipo no.
    cfg::crear_cargo(
        &mut tx,
        &cfg::DatosCargo {
            nombre: "Analista".into(),
            tipo: "administrativo_staff".into(),
        },
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let mut tx = como(&pool, RRHH).await;
    assert!(
        cfg::crear_cargo(
            &mut tx,
            &cfg::DatosCargo {
                nombre: "Analista".into(),
                tipo: "administrativo_staff".into()
            }
        )
        .await
        .unwrap_err()
        .es_duplicado()
    );
    drop(tx);

    // Catálogos: bonos (nombre normalizado; vacío rechazado) y métricas (solo alta).
    let mut tx = como(&pool, RRHH).await;
    let b = cfg::crear_bono(&mut tx, "  Bono   asistencia ")
        .await
        .unwrap();
    assert_eq!(
        cfg::listar_bonos(&mut tx)
            .await
            .unwrap()
            .into_iter()
            .find(|x| x.id == b)
            .unwrap()
            .nombre,
        "Bono asistencia"
    );
    assert_eq!(
        cfg::actualizar_bono(&mut tx, b, "Bono puntualidad")
            .await
            .unwrap(),
        1
    );
    cfg::crear_metrica_catalogo(
        &mut tx,
        &cfg::DatosMetricaCatalogo {
            nombre: "Puntualidad".into(),
            concepto: None,
            formula: Some("a/b".into()),
        },
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let mut tx = como(&pool, RRHH).await;
    let e = cfg::crear_bono(&mut tx, "   ").await.unwrap_err();
    assert_eq!(
        e.check_violado(),
        Some("El nombre del bono es obligatorio.")
    );
    drop(tx);
    let mut tx = como(&pool, RRHH).await;
    let (tipos, modalidades) = cfg::catalogos_servicio(&mut tx).await.unwrap();
    assert!(
        tipos.contains(&"Indefinido".to_owned()) && modalidades.contains(&"Presencial".to_owned())
    );
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
        "revision_matriz"
    )
)]
async fn composicion_metricas_y_escalas(pool: db::PgPool) {
    let mut tx = como(&pool, RRHH).await;
    let cargo = cfg::crear_cargo(
        &mut tx,
        &cfg::DatosCargo {
            nombre: "Coordinador".into(),
            tipo: "operativo".into(),
        },
    )
    .await
    .unwrap();
    let datos = cfg::DatosComposicion {
        haber_basico_diurno: dec!(3200),
        haber_basico_nocturno: Some(dec!(0)),
        cantidad_mujeres: Some(3),
        cantidad_hombres: Some(2),
    };
    let sc = cfg::crear_composicion(&mut tx, uid(CALL_CENTER), cargo, &datos)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    // Misma combinación servicio + cargo: duplicado (UNIQUE de la base).
    let mut tx = como(&pool, RRHH).await;
    assert!(
        cfg::crear_composicion(&mut tx, uid(CALL_CENTER), cargo, &datos)
            .await
            .unwrap_err()
            .es_duplicado()
    );
    drop(tx);

    let mut tx = como(&pool, RRHH).await;
    assert_eq!(
        cfg::actualizar_composicion(
            &mut tx,
            sc,
            &cfg::DatosComposicion {
                haber_basico_diurno: dec!(3300),
                haber_basico_nocturno: None,
                cantidad_mujeres: None,
                cantidad_hombres: None,
            }
        )
        .await
        .unwrap(),
        1
    );
    let metrica = cfg::listar_metricas_catalogo(&mut tx).await.unwrap()[0].id;
    let m = cfg::asignar_metrica(&mut tx, sc, metrica, dec!(100), dec!(150))
        .await
        .unwrap();
    let escalas = vec![
        cfg::UmbralConfig {
            intervalo_min: Some(dec!(0)),
            intervalo_max: Some(dec!(80)),
            pct_bono: dec!(0),
            observaciones: None,
            orden: 1,
        },
        cfg::UmbralConfig {
            intervalo_min: Some(dec!(80)),
            intervalo_max: Some(dec!(100)),
            pct_bono: dec!(100),
            observaciones: Some("completo".into()),
            orden: 2,
        },
    ];
    assert_eq!(
        cfg::actualizar_metrica(&mut tx, m, dec!(100), dec!(200), &escalas)
            .await
            .unwrap(),
        1
    );
    let ms = cfg::metricas_de(&mut tx, sc).await.unwrap();
    assert_eq!(
        (ms.len(), ms[0].monto_bono, ms[0].umbrales.len()),
        (1, dec!(200), 2)
    );
    assert_eq!(ms[0].umbrales[1].observaciones.as_deref(), Some("completo"));
    // Reemplazar escalas por una sola.
    cfg::actualizar_metrica(&mut tx, m, dec!(100), dec!(200), &escalas[..1])
        .await
        .unwrap();
    assert_eq!(
        cfg::metricas_de(&mut tx, sc).await.unwrap()[0]
            .umbrales
            .len(),
        1
    );
    // Quitar la métrica borra sus escalas en cascada.
    assert_eq!(cfg::quitar_metrica(&mut tx, m).await.unwrap(), 1);
    assert!(cfg::metricas_de(&mut tx, sc).await.unwrap().is_empty());
    drop(tx);

    // Estándar no edita composiciones (RLS: 0 filas).
    let mut tx = como(&pool, ESTANDAR).await;
    assert_eq!(
        cfg::actualizar_composicion(&mut tx, sc, &datos)
            .await
            .unwrap(),
        0
    );
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
        "revision_matriz"
    )
)]
async fn composicion_compartida_por_centros(pool: db::PgPool) {
    use domain::composicion_compartida::MontoCentro;
    const OPERADOR: &str = "00000000-0000-0000-0000-000000000062"; // Call center (CC-01) y Campo (CC-02)
    const TECNICO: &str = "00000000-0000-0000-0000-000000000061"; // solo Soporte (CC-01)
    const CC1: &str = "00000000-0000-0000-0000-0000000000c1";
    const CC2: &str = "00000000-0000-0000-0000-0000000000c2";

    let mut tx = como(&pool, RRHH).await;
    assert_eq!(
        cfg::centros_posibles(&mut tx, uid(OPERADOR))
            .await
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        cfg::centros_posibles(&mut tx, uid(TECNICO))
            .await
            .unwrap()
            .len(),
        1
    );
    let pares = [
        MontoCentro {
            centro_costo_id: uid(CC1),
            monto: dec!(3000),
        },
        MontoCentro {
            centro_costo_id: uid(CC2),
            monto: dec!(1000),
        },
    ];
    cfg::guardar_distribucion(&mut tx, uid(OPERADOR), &pares)
        .await
        .unwrap();
    // Reemplaza (no duplica) al volver a guardar.
    cfg::guardar_distribucion(&mut tx, uid(OPERADOR), &pares)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let mut tx = como(&pool, RRHH).await;
    let d = cfg::distribucion(&mut tx, uid(OPERADOR)).await.unwrap();
    assert_eq!(d.len(), 2);
    assert_eq!(
        d.iter().map(|x| x.monto).sum::<rust_decimal::Decimal>(),
        dec!(4000)
    );
    assert_eq!(
        cfg::cargos_con_distribucion(&mut tx).await.unwrap().len(),
        1
    );
    drop(tx);

    // RLS: fuera de RR.HH. no se ve ni se escribe.
    let mut tx = como(&pool, ESTANDAR).await;
    assert!(
        cfg::distribucion(&mut tx, uid(OPERADOR))
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        cfg::guardar_distribucion(&mut tx, uid(TECNICO), &pares)
            .await
            .is_err()
    );
}

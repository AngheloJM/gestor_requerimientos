mod comun;

use comun::*;
use db::requerimientos::listar;
use domain::UsuarioId;

const REQ_81: &str = "00000000-0000-0000-0000-000000000081";
const REQ_84: &str = "00000000-0000-0000-0000-000000000084";

/// Asigna solicitantes y fechas de creación (como superusuario, sin RLS).
async fn preparar(pool: &db::PgPool) {
    sqlx::query(
        "UPDATE requerimientos SET created_at = '2026-10-01'::timestamptz + (numero || ' days')::interval",
    )
    .execute(pool)
    .await
    .unwrap();
    for (usuario, req) in [(ESTANDAR, REQ_84), (RRHH, REQ_81)] {
        sqlx::query("UPDATE requerimientos SET solicitante_id = $1 WHERE id = $2")
            .bind(uid(usuario))
            .bind(uid(req))
            .execute(pool)
            .await
            .unwrap();
    }
}

async fn ids(pool: &db::PgPool, usuario: &str, mios: bool) -> Vec<String> {
    let mut tx = como(pool, usuario).await;
    let filtro = mios.then(|| UsuarioId(uid(usuario)));
    listar(&mut tx, filtro)
        .await
        .unwrap()
        .into_iter()
        .map(|r| uuid::Uuid::from(r.id).to_string())
        .collect()
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(migrations = false, fixtures("supabase_subset", "rls_test", "datos"))]
async fn rrhh_ve_todos_los_activos_del_mas_reciente_al_mas_antiguo(pool: db::PgPool) {
    preparar(&pool).await;
    let v = ids(&pool, RRHH, false).await;
    assert_eq!(
        v.len(),
        5,
        "6 requerimientos menos el del servicio inactivo"
    );
    assert!(
        !v.iter().any(|id| id.ends_with("85")),
        "servicio inactivo excluido"
    );
    assert!(v.first().unwrap().ends_with("86"), "más reciente primero");
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(migrations = false, fixtures("supabase_subset", "rls_test", "datos"))]
async fn mios_filtra_por_solicitante_dentro_de_lo_visible(pool: db::PgPool) {
    preparar(&pool).await;
    assert_eq!(ids(&pool, RRHH, true).await, [REQ_81]);
    assert_eq!(ids(&pool, ESTANDAR, true).await, [REQ_84]);
    // ESTANDAR solo ve su servicio (RLS); COSTOS ve Call center pero no pidió nada.
    assert_eq!(ids(&pool, ESTANDAR, false).await, [REQ_84]);
    assert_eq!(ids(&pool, COSTOS, false).await.len(), 3);
    assert!(ids(&pool, COSTOS, true).await.is_empty());
}

// ---------------------------------------------------------------------------------------
// Creación: usa la copia textual de `crear_requerimiento_con_snapshot` (fixture p2_creacion).
// ---------------------------------------------------------------------------------------

use chrono::NaiveDate;
use db::{
    DbError,
    requerimientos::{crear, opciones},
};
use domain::{BonoExtra, CargoId, NuevoRequerimiento, ServicioId};
use rust_decimal_macros::dec;

const SOPORTE: &str = "00000000-0000-0000-0000-000000000051";
const CALL_CENTER: &str = "00000000-0000-0000-0000-000000000052";
const TECNICO: &str = "00000000-0000-0000-0000-000000000061";
const OPERADOR: &str = "00000000-0000-0000-0000-000000000062";
const EXTERNO: &str = "00000000-0000-0000-0000-aaaa00000005";

fn f(m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, m, d).unwrap()
}

fn nuevo(servicio: &str, cargo: &str) -> NuevoRequerimiento {
    NuevoRequerimiento {
        tipo: "nuevo".into(),
        servicio_id: Some(ServicioId(uid(servicio))),
        cargo_id: CargoId(uid(cargo)),
        cantidad: 2,
        motivo: "Crecimiento".into(),
        fecha_inicio_capacitacion: f(12, 1),
        fecha_inicio_operacion: f(12, 15),
        fecha_fin: None,
        comentarios: Some("  turno tarde  ".into()),
        bonos_extra: vec![],
    }
}

async fn crear_como(
    pool: &db::PgPool,
    usuario: &str,
    n: &NuevoRequerimiento,
) -> Result<String, DbError> {
    let mut tx = como(pool, usuario).await;
    let r = crear(&mut tx, n).await;
    if r.is_ok() {
        tx.commit().await.unwrap();
    }
    r
}

fn regla(r: Result<String, DbError>) -> String {
    r.unwrap_err()
        .regla_de_negocio()
        .expect("regla de negocio")
        .to_owned()
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(
    migrations = false,
    fixtures(
        "supabase_subset",
        "rls_test",
        "datos",
        "p2_edicion",
        "p2_creacion",
        "revision_matriz"
    )
)]
async fn opciones_segun_servicios_del_usuario(pool: db::PgPool) {
    let mut tx = como(&pool, RRHH).await;
    let o = opciones(&mut tx).await.unwrap();
    let nombres: Vec<_> = o.servicios.iter().map(|s| s.nombre.as_str()).collect();
    assert!(!nombres.contains(&"Legado"), "servicio inactivo excluido");
    assert!(
        !nombres.contains(&"Instalaciones"),
        "sin cargos configurados"
    );
    let call = o
        .servicios
        .iter()
        .find(|s| s.nombre == "Call center")
        .unwrap();
    let cargos: Vec<_> = call.cargos.iter().map(|c| c.nombre.as_str()).collect();
    assert_eq!(cargos, ["Agente", "Operador"]);
    drop(tx);

    let mut tx = como(&pool, ESTANDAR).await;
    let o = opciones(&mut tx).await.unwrap();
    let nombres: Vec<_> = o.servicios.iter().map(|s| s.nombre.as_str()).collect();
    assert_eq!(nombres, ["Soporte"], "solo servicios asignados");
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(
    migrations = false,
    fixtures(
        "supabase_subset",
        "rls_test",
        "datos",
        "p2_edicion",
        "p2_creacion",
        "revision_matriz"
    )
)]
async fn crea_pendiente_con_snapshot_y_bonos_extra(pool: db::PgPool) {
    let mut n = nuevo(SOPORTE, TECNICO);
    n.bonos_extra = vec![BonoExtra {
        nombre: "Movilidad".into(),
        monto: dec!(150.50),
        motivo: "Zona alejada".into(),
    }];
    let folio = crear_como(&pool, ESTANDAR, &n).await.unwrap();
    assert!(
        folio.starts_with("REQ-"),
        "folio generado por la base: {folio}"
    );

    let (sol, estado, haber, bonos, coment, cap): (
        uuid::Uuid,
        String,
        rust_decimal::Decimal,
        serde_json::Value,
        Option<String>,
        NaiveDate,
    ) = sqlx::query_as(
        "SELECT solicitante_id, estado, haber_basico_diurno_snap, bonos_snap,
                comentarios_adicionales, fecha_inicio_capacitacion
         FROM requerimientos WHERE folio = $1",
    )
    .bind(&folio)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(sol, uid(ESTANDAR));
    assert_eq!(estado, "pendiente");
    assert_eq!(haber, dec!(2500.00), "sueldo congelado del servicio+cargo");
    assert_eq!(coment.as_deref(), Some("turno tarde"));
    assert_eq!(cap, f(12, 1), "cargo sin tipo: se respeta la fecha enviada");
    assert_eq!(
        bonos,
        serde_json::json!([{"tipo": "extra", "nombre": "Movilidad", "monto": 150.50, "motivo": "Zona alejada"}])
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
        "p2_creacion",
        "revision_matriz"
    )
)]
async fn capacitacion_se_calcula_hacia_atras(pool: db::PgPool) {
    sqlx::query("UPDATE cargos SET tipo = 'operativo' WHERE id = $1")
        .bind(uid(TECNICO))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE servicios SET tiempo_capacitacion_dias = 10 WHERE id = $1")
        .bind(uid(SOPORTE))
        .execute(&pool)
        .await
        .unwrap();
    let folio = crear_como(&pool, ESTANDAR, &nuevo(SOPORTE, TECNICO))
        .await
        .unwrap();
    let cap: NaiveDate =
        sqlx::query_scalar("SELECT fecha_inicio_capacitacion FROM requerimientos WHERE folio = $1")
            .bind(&folio)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(cap, f(12, 5), "15/12 − 10 días");
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(
    migrations = false,
    fixtures(
        "supabase_subset",
        "rls_test",
        "datos",
        "p2_edicion",
        "p2_creacion",
        "revision_matriz"
    )
)]
async fn reglas_de_creacion_de_la_base(pool: db::PgPool) {
    assert_eq!(
        regla(crear_como(&pool, ESTANDAR, &nuevo(CALL_CENTER, OPERADOR)).await),
        "No tienes acceso a ese servicio activo."
    );
    assert_eq!(
        regla(crear_como(&pool, EXTERNO, &nuevo(SOPORTE, TECNICO)).await),
        "Tu rol no permite crear requerimientos."
    );
    let mut n = nuevo(SOPORTE, TECNICO);
    n.motivo = "Otro".into();
    assert_eq!(
        regla(crear_como(&pool, ESTANDAR, &n).await),
        "Elige un motivo."
    );
    let mut n = nuevo(SOPORTE, OPERADOR);
    n.tipo = "nuevo".into();
    assert_eq!(
        regla(crear_como(&pool, RRHH, &n).await),
        "Ese cargo no pertenece al servicio elegido."
    );
}

#[ignore = "requiere DATABASE_URL (Postgres de pruebas)"]
#[sqlx::test(
    migrations = false,
    fixtures("supabase_subset", "rls_test", "datos", "p2_edicion")
)]
async fn detalle_respeta_rls_y_servicio_activo(pool: db::PgPool) {
    // Preparación como superusuario y sin triggers (los de producción protegen los snapshots).
    let mut prep = pool.begin().await.unwrap();
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *prep)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE requerimientos SET bonos_snap = '[{\"metrica\":\"Calidad\",\"monto\":150}]',
         haber_basico_diurno_snap = 3000 WHERE id = $1",
    )
    .bind(uid(REQ_81))
    .execute(&mut *prep)
    .await
    .unwrap();
    prep.commit().await.unwrap();
    let detalle = |usuario: &'static str, req: &'static str| {
        let pool = pool.clone();
        async move {
            let mut tx = como(&pool, usuario).await;
            db::requerimientos::detalle(&mut tx, uid(req))
                .await
                .unwrap()
        }
    };
    // RR.HH. ve el detalle completo, con datos del servicio y lo congelado.
    let d = detalle(RRHH, REQ_81).await.expect("visible para RR.HH.");
    assert_eq!(d.servicio_nombre.as_deref(), Some("Call center"));
    assert_eq!(d.cargo_nombre.as_deref(), Some("Operador"));
    assert_eq!(d.centro_costo.as_deref(), Some("CC-01 · Centro Uno"));
    assert_eq!(d.cantidad, Some(3));
    assert_eq!(
        d.haber_basico_diurno,
        Some(rust_decimal::Decimal::from(3000))
    );
    assert_eq!(d.bonos.unwrap()[0]["metrica"], "Calidad");
    // Servicio inactivo: no se muestra (como en el listado).
    assert!(
        detalle(RRHH, "00000000-0000-0000-0000-000000000085")
            .await
            .is_none()
    );
    // Sin servicios asignados y sin ser solicitante: RLS no lo deja ver.
    assert!(detalle(SIN_SERVICIOS, REQ_81).await.is_none());
}

# Pendientes y supuestos

Actualizado con el schema real de Supabase (inspección directa del proyecto).
`TODO(negocio)` = requiere decisión del cliente; `TODO(verificar)` = comprobar en la base real.

## Decisiones de negocio pendientes (no implementadas a criterio propio)

- [ ] **Resultado mensual de métricas.** No existe la tabla ni está definido quién/cómo lo carga.
      Mientras tanto `total_ganado` = haber diurno + nocturno + suma de `monto_bono` configurados
      (regla confirmada, igual a la aplicación original), **sin ajustar por cumplimiento real**:
      revisar cuando exista la tabla (umbrales y escala de `pct_bono`, 0–100 o 0–1).
      → `ComposicionSalarial::total_ganado`, `MetricaBono::umbral_para` (`domain/src/composicion_salarial.rs`)
- [ ] **`catalogo_bonos`**: 2 filas en producción y **ninguna FK la referencia** (sin conectar).
      No portar como fuente de bonos hasta definir su uso.
- [ ] **`/operadores`** — políticas RLS verificadas en la base real:
      - `operadores_select_authenticated` (SELECT): `current_user_role() IN ('rrhh_admin','encargado_costos')`;
      - `operadores_write_rrhh_admin` (ALL): `is_rrhh_admin(auth.uid())`.
      `estandar` y `externo` **no** leen la tabla. Pendiente: si `estandar` ve algún resumen, de qué
      consulta agregada sale. Verificado: `current_user_role()` es
      `SELECT rol FROM usuarios WHERE id = auth.uid()` (SECURITY DEFINER), equivalente a
      `test_rol()` del fixture `rls_test.sql`; funciona con los claims que setea `db::rls`.
- [ ] **`usuarios.es_reclutador_staff_default`** y su relación con `requerimientos.reclutador_asignado_id`:
      sin definir. No se tocó ni se modeló.
- [ ] **¿`estandar` ve montos salariales de su servicio o solo la estructura?** No confirmado.
      Hoy la matriz devuelve lo que RLS permite ver. RLS filtra **filas**, no columnas: ocultar
      montos requeriría una vista o privilegios por columna (cambio de base, a decidir).
- [ ] **Categoría de desvío** (`falla_capacitacion`, `rechazo_operaciones`, `ratio_reclutamiento`,
      `cambio_cliente`): la API la acepta (opcional, junto con su explicación); falta definir
      cuándo la interfaz debe pedirla.
- [ ] **Kanban: enrutamiento de `solicitudes_cambio.campo` a las columnas Comercial y Operaciones.**
      Solo se conoce: por defecto RR.HH. (+ solicitante); `campo = 'centro_costo'` también Costos
      (visibilidad que aplica RLS). Hoy todas las tarjetas van a la columna RR.HH.
      → `area_kanban` (`domain/src/tablero.rs`)
- [ ] **Kanban: acciones** (sin implementar). Reglas verificadas en producción (CHECK +
      trigger `validar_solicitud_cambio`):
      - estados: `pendiente`, `resuelta`, `rechazada`, `cancelada`; solo se cierra desde `pendiente`;
      - Resolver → `resuelta` / Rechazar → `rechazada`: RR.HH., o Costos si `campo = 'centro_costo'`;
        comentario de resolución obligatorio (≥ 3); la base fija `responsable_id` y `cerrada_at`;
      - Cancelar → `cancelada`: solo quien la pidió; comentario opcional;
      - "Ir a corregir": no existe un estado intermedio → solo navega (no cambia estado);
      - `campo` ∈ centro_costo, cliente, contrato_comercial, tipo_contrato, modalidad,
        dias_trabajo, sueldo_basico, bonos (los dos últimos exigen `cargo_id`).
- [ ] **Kanban: ¿mostrar solicitudes cerradas** (`cerrada_at` no nulo)? Hoy se devuelven todas.
- [ ] **Completitud: criterio de "composición salarial completa".** Cada fila de `servicio_cargos`
      ya exige `haber_basico_diurno`; no está definido qué la hace incompleta. No se evalúa
      (`criterios_sin_definir` en `/tablero`).
- [ ] **Completitud: mapeo de tareas a áreas** (contrato → Comercial, cargos → Operaciones) es un
      supuesto previo, no confirmado.
- [ ] **Contratos con duración** (`duracion_valor`/`duracion_unidad`): falta la semántica de
      `duracion_unidad` para calcular el fin. Hoy su vigencia es "indeterminada"
      (tarea `verificar_vigencia_contrato`). Lo mismo si no tiene ni fecha_fin ni duración.
- [x] **Almanaque** (2026-11-03): usa `leer_eventos_almanaque` de producción, igual que la app
      original → no filtra por estado del requerimiento; entregas/rectificaciones según
      `requerimiento_entregas.tipo`, con estado de decisión (pendiente/aceptada/rechazada);
      acceso: RR.HH. todo, costos/estándar sus servicios asignados, otros 403.
      Se agrega el filtro propio de **servicios activos** (cruce por folio).
- [ ] **Almanaque — pendientes de negocio:**
      - si un requerimiento con `cantidad_personas` NULL cuenta como 1 (hoy se informa aparte);
      - confirmar que la rectificación es un ajuste (CHECK `cantidad <> 0` admite negativos):
        hoy se muestra aparte, con su signo, sin netear;
      - diferencia con RLS: la función no muestra al solicitante ni al inmediato superior los
        requerimientos de servicios que no tienen asignados (sí los ven en "Mis requerimientos").
- [ ] **Filas de `servicio_cargos` con `servicio_id` NULL:** qué representan. La matriz las excluye.
- [ ] **Overrides por operador** (`operador_composicion_override`): no se usan aún en ningún endpoint.

## Verificaciones técnicas contra la base real

- [ ] **Rol de conexión:** el usuario de `DATABASE_URL` debe poder `SET ROLE authenticated`
      (la API lo verifica al arrancar).
- [ ] **Políticas RLS reales:** confirmar que permiten a RR.HH. `UPDATE requerimientos` e
      `INSERT requerimiento_historial`, y si el solicitante ve el servicio de su solicitud de cambio
      (si no, ver el punto siguiente).
- [ ] **Kanban y servicios no visibles:** si RLS oculta el servicio de una solicitud, no se puede
      saber si está activo; hoy la tarjeta se muestra (con nombre de servicio vacío).
- [ ] **Tipos de columnas:** se asumió `uuid` en todas las PK/FK; montos, porcentajes y fechas se
      castean en SQL. Si `fecha_*` son `timestamptz`, definir la zona horaria de corte del día.
- [x] **Conexión real:** pooler de Supabase en modo sesión (puerto 5432), usuario `postgres.<ref>`.
- [ ] **Pooler:** usar puerto 5432. Con modo transacción (6543), además de la transacción explícita
      (ya implementada), revisar el uso de sentencias preparadas de SQLx.
- [ ] **Baseline:** pegar el `pg_dump --schema-only --schema=public` real en
      `backend/migrations/0001_init.sql` y marcarla como aplicada en Supabase sin ejecutarla.

## Autenticación propia

- [ ] **Migración `0002_autenticacion_propia.sql` (tablas `credenciales` y `sesiones`)**: aditiva,
      aplicada solo en local. **No aplicada en Supabase**: requiere confirmación.
- [ ] **Usuarios reales**: hoy inician sesión con Supabase Auth (contraseñas en `auth.users`, bcrypt).
      Con el login propio cada usuario necesita una credencial nueva (`establecer-credenciales`).
      TODO(negocio): ¿se les asigna contraseña inicial / flujo de "olvidé mi contraseña"?
      (Opción técnica: verificar el bcrypt de `auth.users` en el primer login y migrar a argon2id.)
- [ ] **Recuperación de contraseña** (email): no implementada (requiere servicio de correo).
- [ ] **Limitador de intentos en memoria**: válido para una sola instancia; con varias réplicas,
      moverlo a Redis o a una tabla. Detrás de un proxy, configurar IP real (`X-Forwarded-For`).
- [ ] **Contraseñas filtradas**: validar contra listas conocidas (HIBP k-anonymity).
- [ ] **HTTPS en producción** + `HSTS=true`.

## Roles y permisos

- [ ] **Migración `0003_roles_permisos.sql`** (aditiva): aplicada solo en local. Aplicarla en
      Supabase requiere confirmación. Ver `docs/PLAN_ROLES.md` (fases 2–4 y preguntas abiertas).
- [x] Administración de roles: solo `rrhh_admin`. Un usuario = un solo rol (decisión 2026-11-03).
- [ ] TODO(negocio): si `estandar` ve montos (`sueldos.ver_montos`; hoy RLS se los muestra).

## Usuarios

- [ ] **Alta de usuarios desde la app**: no disponible. En producción `usuarios.id` es FK a
      `auth.users` (Supabase Auth, ON DELETE CASCADE). TODO(negocio): crear la cuenta en
      Supabase Auth (Admin API, requiere la service key en el backend) o quitar esa FK
      (cambio no aditivo). Hoy: alta en Supabase + `establecer-credenciales` (CLI).
- [ ] **Autoedición**: la pantalla no deja a un RR.HH. cambiar su propio rol ni desactivarse
      (protección de interfaz para no perder el acceso). Confirmar si se mantiene.
- [ ] Edición de `nombre` y de `es_reclutador_staff_default`: fuera de la pantalla (igual que
      el original; el segundo sigue sin definir).

## Frontend

- [ ] Copiar el código de Lovable y portar las pantallas de matriz, almanaque y aprobaciones.
- [ ] Generar los tipos de la API desde OpenAPI en lugar de mantenerlos a mano.

## Resuelto

- [x] **Pantalla de aprobación: solo `rrhh_admin`** (decisión 2026-11-03). El resto de los usuarios
      consulta en "Mis requerimientos" si su requerimiento fue aprobado o no (`estado`).
      La autorización sigue en la base (`p2_acceso`); la restricción de la pantalla es de interfaz.

- [x] **Edición de requerimientos** (verificado en producción 2026-11-03): el PATCH llama a
      `editar_requerimiento_con_historial` (glosa obligatoria, versión `updated_at`, desvío);
      el historial lo escribe el trigger `p2_auditar_requerimiento` (se quitó el INSERT propio,
      que duplicaba y además fallaba por la glosa obligatoria). Copia textual en el fixture
      `p2_edicion.sql`. Fechas del historial: `YYYY-MM-DD` (las escribe la base).
- [x] **Reglas de la base para requerimientos:** `estado` ∈ pendiente, en_proceso, aprobado,
      rechazado, cerrado; `motivo` ∈ Renuncia, Baja médica, Nuevo proyecto, Reposición, Crecimiento.
- [x] **Parámetros salariales:** claves `salario_minimo_nacional` (vigente Bs 3600 desde
      2026-10-28) y `recargo_nocturno_pct` (0.25). Vigente = mayor `vigente_desde <= hoy`.

- [x] Login propio con argon2id (reemplaza Supabase Auth; el código JWKS quedó en el historial de git).
- [x] Autorización: delegada a RLS (`SET LOCAL ROLE authenticated` + `request.jwt.claims` por
      transacción). Se eliminó toda la autorización reimplementada en Rust.
- [x] Schema real aplicado a dominio y consultas; `servicios.activo = true` en todas las consultas.
- [x] Orden de la matriz Centro de Costo → Cliente → Servicio → Cargo (decisión de negocio).

# Pendientes y supuestos

Actualizado con el schema real de Supabase (inspección directa del proyecto).
`TODO(negocio)` = requiere decisión del cliente; `TODO(verificar)` = comprobar en la base real.

## Decisiones de negocio pendientes (no implementadas a criterio propio)

- [ ] **Resultado mensual de métricas.** No existe la tabla ni está definido quién/cómo lo carga.
      Sin eso no se puede aplicar `servicio_cargo_metrica_umbrales` ni calcular "bonos aplicables":
      `total_ganado` se devuelve `null` en `/matriz-servicio` (sí se devuelve `total_haberes`).
      También falta confirmar la escala de `pct_bono` (0–100 o 0–1).
      → `ComposicionSalarial::total_ganado`, `MetricaBono::umbral_para` (`domain/src/composicion_salarial.rs`)
- [ ] **`usuarios.es_reclutador_staff_default`** y su relación con `requerimientos.reclutador_asignado_id`:
      sin definir. No se tocó ni se modeló.
- [ ] **¿`estandar` ve montos salariales de su servicio o solo la estructura?** No confirmado.
      Hoy la matriz devuelve lo que RLS permite ver. RLS filtra **filas**, no columnas: ocultar
      montos requeriría una vista o privilegios por columna (cambio de base, a decidir).
- [ ] **Kanban: enrutamiento de `solicitudes_cambio.campo` a las columnas Comercial y Operaciones.**
      Solo se conoce: por defecto RR.HH. (+ solicitante); `campo = 'centro_costo'` también Costos
      (visibilidad que aplica RLS). Hoy todas las tarjetas van a la columna RR.HH.
      → `area_kanban` (`domain/src/tablero.rs`)
- [ ] **Kanban: ¿mostrar solicitudes cerradas** (`cerrada_at` no nulo)? Hoy se devuelven todas.
- [ ] **Completitud: criterio de "composición salarial completa".** Cada fila de `servicio_cargos`
      ya exige `haber_basico_diurno`; no está definido qué la hace incompleta. No se evalúa
      (`criterios_sin_definir` en `/tablero`).
- [ ] **Completitud: mapeo de tareas a áreas** (contrato → Comercial, cargos → Operaciones) es un
      supuesto previo, no confirmado.
- [ ] **Contratos con duración** (`duracion_valor`/`duracion_unidad`): falta la semántica de
      `duracion_unidad` para calcular el fin. Hoy su vigencia es "indeterminada"
      (tarea `verificar_vigencia_contrato`). Lo mismo si no tiene ni fecha_fin ni duración.
- [ ] **Almanaque:**
      - qué `requerimientos.estado` y qué decisiones de entrega (`requerimiento_entrega_decisiones`) cuentan
        (hoy no se filtra por estado);
      - rol de `requerimiento_entregas.tipo` (hoy entrega/rectificación se distingue solo por `corrige_entrega_id`);
      - si la cantidad de una rectificación reemplaza o ajusta la de la entrega corregida (hoy se lista aparte);
      - si un requerimiento con `cantidad_personas` NULL cuenta como 1 (hoy no se suma; se informa
        `registros_sin_cantidad`).
- [ ] **`historial`: uso de `grupo_edicion`, `categoria_desvio`, `detalle_desvio`.** El PATCH solo
      escribe `campo`, valores, `usuario_id` y `glosa` (opcional).
- [ ] **Filas de `servicio_cargos` con `servicio_id` NULL:** qué representan. La matriz las excluye.
- [ ] **Overrides por operador** (`operador_composicion_override`): no se usan aún en ningún endpoint.

## Verificaciones técnicas contra la base real

- [ ] **Rol de conexión:** el usuario de `DATABASE_URL` debe poder `SET ROLE authenticated`
      (la API lo verifica al arrancar).
- [ ] **Trigger de historial:** si un trigger ya escribe `requerimiento_historial` al actualizar
      `requerimientos`, el INSERT explícito del PATCH lo duplicaría → quitar el INSERT.
- [ ] **Formato de fechas en `requerimiento_historial.valor_*`** usado por Lovable (aquí: `YYYY-MM-DD`).
- [ ] **Políticas RLS reales:** confirmar que permiten a RR.HH. `UPDATE requerimientos` e
      `INSERT requerimiento_historial`, y si el solicitante ve el servicio de su solicitud de cambio
      (si no, ver el punto siguiente).
- [ ] **Kanban y servicios no visibles:** si RLS oculta el servicio de una solicitud, no se puede
      saber si está activo; hoy la tarjeta se muestra (con nombre de servicio vacío).
- [ ] **Tipos de columnas:** se asumió `uuid` en todas las PK/FK; montos, porcentajes y fechas se
      castean en SQL. Si `fecha_*` son `timestamptz`, definir la zona horaria de corte del día.
- [ ] **Pooler:** usar puerto 5432. Con modo transacción (6543), además de la transacción explícita
      (ya implementada), revisar el uso de sentencias preparadas de SQLx.
- [ ] **Baseline:** pegar el `pg_dump --schema-only --schema=public` real en
      `backend/migrations/0001_init.sql` y marcarla como aplicada en Supabase sin ejecutarla.
- [ ] **Probar con el proyecto real:** `SUPABASE_URL=https://<ref>.supabase.co cargo test -p api
      -- --ignored jwks_real`, luego login real y `/me`.
- [ ] **Revocar la clave HS256** en Supabase > JWT Keys (ya no debería haber tokens HS256 vigentes).

## Frontend

- [ ] Copiar el código de Lovable y portar las pantallas de matriz, almanaque y aprobaciones.
- [ ] Confirmar el método de login de Lovable (hoy: email + contraseña).
- [ ] Generar los tipos de la API desde OpenAPI en lugar de mantenerlos a mano.

## Resuelto

- [x] JWT: ES256 vía JWKS con `kid`, refresco periódico y por rotación, validación de `iss`;
      HS256 solo como fallback opcional (desactivado por defecto).
- [x] Autorización: delegada a RLS (`SET LOCAL ROLE authenticated` + `request.jwt.claims` por
      transacción). Se eliminó toda la autorización reimplementada en Rust.
- [x] Schema real aplicado a dominio y consultas; `servicios.activo = true` en todas las consultas.
- [x] Orden de la matriz Centro de Costo → Cliente → Servicio → Cargo (decisión de negocio).

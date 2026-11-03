# Inventario de pantallas y campos

Referencia: capturas de la versión original (Lovable). Solo se registran **pantallas y campos**;
el diseño visual es propio (Sistema NOVA). Columnas = nombres del schema real cuando se conocen.
Leyenda: ✅ ya expuesto por la API · 🟡 columna existe, falta endpoint/pantalla · ❌ sin columna conocida.

## Navegación

- Operación: **Almanaque**, **Mis requerimientos**.
- Configuración: **Matriz de servicio**, **Parámetros salariales**, **Tablero**, **Usuarios**.
- Pie de la barra lateral: nombre del usuario, su cargo ("Cargo sin asignar" si no tiene), cerrar sesión.
- Rutas adicionales: `/aprobaciones`, `/nuevo-requerimiento`, `/operadores`, `/cuenta-no-configurada`.
- **`/operadores`**: listado de empleados (nombre, documento, cargo, servicio, fecha de ingreso),
  ligado a `operador_composicion_override` (sueldos de excepción). RLS verificada: leen
  `rrhh_admin` y `encargado_costos`; solo `rrhh_admin` escribe. El menú no debe mostrar la
  opción a otros roles. TODO(verificar): si `estandar` ve un resumen, de qué consulta sale.
- **`/cuenta-no-configurada`**: aterrizaje para quien inició sesión sin fila completa en `usuarios`
  ("Cargo sin asignar"): mensaje para contactar a RR.HH. en lugar de un 403.

## Tablero de pendientes ✅ (parcial)

- Indicadores: servicios configurados, servicios incompletos, áreas sin completar su tarea.
- Kanban de 3 columnas: **Comercial**, **Operaciones**, **RR.HH.**, cada una con "N pendientes"
  y estado vacío ("Sin pendientes…").
- Tarjeta (`solicitudes_cambio`): cliente, servicio · campo, cargo, descripción/motivo,
  estado · fecha.
- Acciones de la tarjeta: **Ir a corregir**, **Resolver**, **Rechazar**, **Cancelar solicitud**. ❌ sin endpoints.
  TODO(negocio): transiciones de `estado` y quién puede ejecutar cada acción.

## Mis requerimientos 🟡

- RR.HH. ve "Todos los requerimientos" (lo filtra RLS); el resto, los suyos.
- Botones: **Aprobación de requerimientos** (→ `/aprobaciones`, **solo `rrhh_admin`**),
  **Crear requerimiento** (→ `/nuevo-requerimiento`).
- Cada usuario ve aquí si su requerimiento fue aprobado o no (`estado`: pendiente, en_proceso,
  aprobado, rechazado, cerrado).
- Tarjeta: `folio` (formato `REQ-000017`), servicio, cargo, `tipo`, `estado` (badge),
  `fecha_inicio_operacion` ("Sin informar" si es nula), `created_at`.
- TODO(verificar): quién genera `folio` (¿trigger en la base?).

## Matriz de servicio 🟡

Orden obligatorio (decisión cerrada con la clienta): **Centro de Costo → Cliente → Servicio → Cargo**.
Proyecto solo como dato adicional o filtro, nunca antes de Centro de Costo.

Botón **Carga rápida**: asistente (4 pasos en el original) para dar de alta un servicio con sus
cargos y sueldos de una vez. En el original no carga métricas/bonos, no tiene revisión final ni
guarda borrador. TODO(negocio): definir si el nuestro suma esos pasos. Secciones numeradas con contador:

1. **Clientes y proyectos** — cliente: nombre, `logo_url`, `manual_marca_url`, editar/eliminar.
   Proyectos: lista propia ("Nuevo proyecto").
2. Centros de costo · 3. Servicios · 4. Cargos · 6. Contratos
5. **Composición salarial** ✅ (lectura) — ver abajo.
7. **Métricas y bonos por cargo** (contador de configurados y de pendientes).
8. Catálogo de métricas · 9. **Catálogo de bonos** (`catalogo_bonos`: id, nombre; sin tabla
   puente conocida — TODO(verificar) si está en uso)
10. **Pendientes de completar**.

TODO(negocio): editar/eliminar en la matriz requiere confirmar políticas RLS de escritura y
si "eliminar" es borrado lógico (`activo = false`) — los cambios de base son solo aditivos.

### Composición salarial

- Filtros: servicio, cliente, proyecto, centro de costo + búsqueda libre
  (cliente, proyecto, centro, servicio, cargo o contrato).
- Tarjeta: cliente, proyecto, código CC, centro de costo, servicio (código - nombre), cargo,
  haber básico diurno, haber básico nocturno, **un renglón por bono/métrica** (monto o "—"),
  **total ganado**, contrato comercial, editar.
- Alerta "Por debajo del Salario Mínimo Nacional vigente (Bs X)", desde `parametros_salariales_historial`.
- ✅ *Total ganado = haber básico (diurno + nocturno) + suma de bonos configurados* (confirmado;
  sin ajustar por cumplimiento real).

## Parámetros salariales ❌

- Tabla `parametros_salariales_historial(id, parametro, valor, vigente_desde, creado_por, created_at)`:
  cada cambio agrega una fila nueva (no se pisa el valor anterior).
- Parámetro conocido: salario mínimo nacional. Según la reunión: recargo nocturno y feriados.
  TODO(negocio): claves exactas de `parametro`.

## Usuarios 🟡

- Lista con búsqueda y contador. Tarjeta: nombre, cargo, `division`, `region`, `rol`, `activo`, editar.
- Editar: cargo (select de `cargos`), **inmediato superior** (select de `usuarios`), división,
  región, rol (`externo`, `estandar`, `encargado_costos`, `rrhh_admin`), activo (switch).
- **Servicios asignados** (`usuario_servicios`); para `rrhh_admin` se muestra que ve todos por su rol.
- Con el login propio, el alta de usuario necesita además email + contraseña inicial
  (ver PENDIENTES.md, autenticación).

## Confirmado por las capturas

- Los roles de la aplicación son los de la base: `rrhh_admin`, `estandar`, `encargado_costos`, `externo`.
- Un requerimiento tiene **un** cargo (`cargo_id`), como en el schema actual.

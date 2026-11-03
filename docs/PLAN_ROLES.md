# Plan: roles y permisos configurables

Estado: **borrador para definir**. Nada de esto está implementado ni aplicado en la base.
Objetivo: poder crear roles nuevos (ej. "Gerente") y decidir, por módulo y acción, qué puede
**ver**, **crear**, **editar** o **aprobar** cada uno, sin tocar código para cada caso.

## 1. Cómo funciona hoy (verificado en producción, 2026-11-03)

- `usuarios.rol` es texto con un CHECK fijo: `externo`, `estandar`, `encargado_costos`, `rrhh_admin`.
  **Un rol nuevo no se puede ni guardar** sin cambiar ese CHECK.
- Los permisos viven en **56 políticas RLS** que nombran roles concretos mediante funciones
  (`is_rrhh_admin`, `current_user_role`, `es_costos_activo`, `p2_acceso`, `puede_ver_servicio`,
  `tiene_servicio_asignado`) y en triggers (`proteger_campos_requerimiento`,
  `editar_requerimiento_con_historial`, `validar_solicitud_cambio`).
- La base es **compartida con la aplicación original (Lovable)**: cualquier cambio de políticas
  la afecta también.

Resumen de lo que permite hoy cada rol:

| Módulo | rrhh_admin | encargado_costos | estandar | externo |
|---|---|---|---|---|
| Requerimientos: ver | todos | propios, servicios asignados, superior de | propios, servicios asignados, superior de | propios |
| Requerimientos: crear | sí | sí | sí | no |
| Requerimientos: editar (`p2_acceso`, campos limitados salvo RR.HH.) | todos | propios, superior de | propios, superior de, servicios asignados | no |
| Requerimientos: cambiar `estado` (aprobar) | sí | no | no | no |
| Entregas: registrar / decidir | sí / sí | con `p2_acceso` / no | con `p2_acceso` / no | no |
| Sueldos (`servicio_cargos`, métricas, umbrales): ver | todos | según `puede_ver_servicio` | según `puede_ver_servicio` | según `puede_ver_servicio` |
| Sueldos: editar | sí | no | no | no |
| Matriz (clientes, centros, contratos, servicios, cargos): editar | sí | no | no | no |
| Catálogo de bonos: ver | sí | sí | no | no |
| Operadores / overrides: ver · editar | sí · sí | sí · no | no | no |
| Parámetros salariales: ver · editar | sí · sí | sí · no | sí · no | sí · no |
| Solicitudes de cambio: crear | sí | servicios asignados | servicios asignados | no |
| Solicitudes de cambio: resolver | sí | solo `campo = centro_costo` | no | no |
| Usuarios y asignación de servicios: editar | sí | no | no | no |

`puede_ver_servicio` no fue revisada todavía (TODO: leer su definición).

## 2. Propuesta: permisos por rol, guardados en la base

### Modelo (tablas nuevas, cambio aditivo)

```
roles          (id, clave UNIQUE, nombre, descripcion, es_sistema, activo, created_at, updated_at)
permisos       (clave PK, modulo, accion, descripcion)        -- catálogo fijo, lo define el código
rol_permisos   (rol_id, permiso_clave, alcance, PK(rol_id, permiso_clave))
               alcance ∈ 'todos' | 'servicios_asignados' | 'propios'
rol_permisos_historial (quién cambió qué permiso, cuándo; auditoría)
```

- Los 4 roles actuales se cargan como `es_sistema = true` (no se pueden borrar) con los permisos
  que equivalen exactamente a la tabla de arriba.
- Una función `tiene_permiso(permiso text, servicio uuid DEFAULT NULL) RETURNS boolean`
  (STABLE, SECURITY DEFINER) resuelve el permiso del usuario actual (`auth.uid()`), aplicando el
  alcance con `usuario_servicios` / `solicitante_id`.
- La autorización sigue en Postgres (RLS); el backend Rust **no** decide permisos, solo los
  muestra (para armar el menú y habilitar botones).

### Catálogo inicial de permisos (a detallar juntos)

| Módulo | Acciones |
|---|---|
| requerimientos | ver, crear, editar, aprobar (cambiar estado), ver_historial |
| entregas | ver, registrar, decidir |
| sueldos | ver_estructura, ver_montos, editar |
| matriz | ver, editar (clientes, centros, contratos, servicios, cargos) |
| metricas_bonos | ver, editar |
| parametros_salariales | ver, editar |
| operadores | ver, editar |
| solicitudes_cambio | crear, resolver |
| usuarios | ver, editar, asignar_servicios |
| roles | administrar |
| almanaque / tablero | ver |

`sueldos.ver_montos` separado de `ver_estructura` resuelve la duda pendiente de si `estandar`
ve montos (RLS filtra filas, no columnas: requiere una vista o función que oculte montos).

## 3. Fases (cada una requiere su confirmación)

| Fase | Qué cambia | Riesgo para la app original |
|---|---|---|
| **0. Tablas + función + carga de los 4 roles** | Solo agrega objetos. Nada usa todavía los permisos. | Ninguno |
| **1. Pantalla "Roles"** (Configuración) | Matriz de checkboxes por módulo/acción y alcance; asignar rol en "Usuarios". | Ninguno (solo escribe en tablas nuevas) |
| **2. Aceptar roles nuevos en `usuarios.rol`** | Reemplazar el CHECK por una FK a `roles.clave` (o ampliar el CHECK). | Bajo: la app original no conoce los roles nuevos (los trataría como desconocidos) |
| **3. Políticas que consultan `tiene_permiso`** | Por módulo: agregar `OR tiene_permiso(...)` a cada política, **sin quitar** las condiciones actuales. | Medio: cambia políticas en producción; probar módulo por módulo |
| **4. Simplificar** | Cuando se retire la app original, dejar solo `tiene_permiso`. | — |

Las fases 2 y 3 **modifican objetos existentes** (CHECK y políticas): no son puramente aditivas.
Se proponen ampliando (nunca restringiendo lo que hoy funciona) y con respaldo
(`pg_get_constraintdef` / `pg_policy` guardados antes de cada cambio).

## 4. Seguridad

- Solo quien tenga `roles.administrar` edita roles y permisos (lo aplica RLS sobre las tablas nuevas).
- No se puede quitar `roles.administrar` al último rol que lo tenga, ni borrar roles de sistema
  o roles con usuarios asignados (desactivar en su lugar).
- Todo cambio de permisos queda en `rol_permisos_historial` (quién, cuándo, antes/después).
- Los permisos se evalúan en cada consulta (no se cachean en el token): quitar un permiso
  tiene efecto inmediato.

## 5. Preguntas para definir

1. ¿Qué roles nuevos se prevén además de "Gerente"? ¿Un usuario tiene **un** rol o puede tener varios?
2. Gerente: ¿ve **todos** los servicios o solo los asignados? ¿"Aprobar" es cambiar `estado`
   del requerimiento (hoy exclusivo de RR.HH.) o una aprobación previa a la de RR.HH.?
3. Gerente "modifica sueldo": ¿edita directo la composición salarial o crea una solicitud de
   cambio que RR.HH. resuelve (como hoy los demás roles)?
4. ¿Hace falta aprobación en varios niveles (ej. Gerente → RR.HH.)? Eso es un flujo nuevo, no
   solo un permiso.
5. ¿Quién administra roles: solo `rrhh_admin` o un rol "Administrador del sistema" aparte?
6. ¿Hasta cuándo convive la app original? Define si conviene hacer la fase 3 o esperar a la 4.

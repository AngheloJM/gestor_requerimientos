-- =====================================================================================
-- DATOS DE DEMOSTRACIÓN — SOLO BASE LOCAL. NUNCA EJECUTAR EN PRODUCCIÓN.
-- =====================================================================================
-- Ejecutar con `infra/db/seed-demo.sh` (usa solo la URL local de infra/db/db.sh).
-- Reseteable: el primer bloque borra todo lo "Demo" y lo vuelve a crear; se puede correr
-- las veces que haga falta sin duplicar. Explicación de cada caso: docs/DATOS_DE_PRUEBA.md.
--
-- Todos los nombres son FICTICIOS (prefijo "Demo" / código "DEMO-"), sin datos de clientes reales.
--
-- Cómo se crea cada cosa:
--   * Catálogo (cliente, centros, contrato, servicios, cargos, sueldos, tiempos, reparto de
--     cargo compartido, feriado): INSERT directo como superusuario local.
--   * Requerimientos, cambios de cantidad / fecha / estado: con las MISMAS funciones de la base
--     que usa la API (`crear_requerimiento_con_snapshot`, `editar_requerimiento_con_historial`),
--     identificados como el usuario RRHH (igual que hace el backend: rol `authenticated` +
--     claims del JWT). Pasan por todas las validaciones, triggers e historial.
--   * Entregas y rectificación: INSERT directo (la app no tiene pantalla para cargarlas y la
--     base local no tiene función para hacerlo). Se respetan las reglas de producción.
--
-- Cada paso va en su propia transacción para que el historial quede con horas distintas
-- (dentro de una misma transacción `now()` no cambia y el orden saldría mezclado).
-- =====================================================================================

\set ON_ERROR_STOP on

-- Resguardo: la tabla `credenciales` (migración 0002) solo existe en la base local.
DO $$ BEGIN
  IF to_regclass('public.credenciales') IS NULL THEN
    RAISE EXCEPTION 'Esta base no parece la local (falta public.credenciales). Abortado.';
  END IF;
END $$;

-- -------------------------------------------------------------------------------------
-- 0. Limpieza de una corrida anterior (sin disparar triggers de auditoría/protección).
--    También borra la variante local con nombres reales (seed_demo_real.sql, ids ...e0xx y
--    marca "[CASO N]") para que en pantalla haya un solo juego de demostración.
-- -------------------------------------------------------------------------------------
BEGIN;
SET LOCAL session_replication_role = replica;
CREATE TEMP TABLE demo_req ON COMMIT DROP AS
  SELECT r.id FROM requerimientos r
   WHERE r.comentarios_adicionales LIKE '[DEMO %' OR r.comentarios_adicionales LIKE '[CASO %'
      OR r.servicio_id IN (SELECT id FROM servicios WHERE codigo IN ('DEMO-ATE', 'DEMO-COB'))
      OR r.servicio_id = '00000000-0000-4000-8000-00000000e031'
      OR r.cargo_id::text LIKE '00000000-0000-4000-8000-00000000e0%';
DELETE FROM requerimiento_entrega_decisiones
 WHERE entrega_id IN (SELECT id FROM requerimiento_entregas WHERE requerimiento_id IN (SELECT id FROM demo_req));
DELETE FROM requerimiento_entregas WHERE corrige_entrega_id IS NOT NULL AND requerimiento_id IN (SELECT id FROM demo_req);
DELETE FROM requerimiento_entregas WHERE requerimiento_id IN (SELECT id FROM demo_req);
DELETE FROM requerimiento_historial WHERE requerimiento_id IN (SELECT id FROM demo_req);
DELETE FROM requerimiento_referencia WHERE requerimiento_id IN (SELECT id FROM demo_req);
DELETE FROM requerimientos WHERE id IN (SELECT id FROM demo_req);
DELETE FROM cargo_centros_costo
 WHERE cargo_id IN (SELECT id FROM cargos WHERE nombre IN ('Agente Demo', 'Asistente Demo', 'Supervisor Demo Compartido'))
    OR cargo_id::text LIKE '00000000-0000-4000-8000-00000000e0%';
-- Con session_replication_role = replica tampoco corren los ON DELETE CASCADE: las tablas
-- hijas se borran a mano (incluye filas huérfanas que pudiera haber dejado una corrida previa).
DELETE FROM servicio_cargo_metrica_umbrales
 WHERE servicio_cargo_metrica_id IN (
   SELECT m.id FROM servicio_cargo_metricas m
    WHERE m.servicio_cargo_id IN (SELECT sc.id FROM servicio_cargos sc JOIN servicios s ON s.id = sc.servicio_id
                                   WHERE s.codigo IN ('DEMO-ATE', 'DEMO-COB'))
       OR m.servicio_cargo_id::text LIKE '00000000-0000-4000-8000-00000000d0%'
       OR m.servicio_cargo_id::text LIKE '00000000-0000-4000-8000-00000000e0%');
DELETE FROM servicio_cargo_metricas
 WHERE servicio_cargo_id IN (SELECT sc.id FROM servicio_cargos sc JOIN servicios s ON s.id = sc.servicio_id
                              WHERE s.codigo IN ('DEMO-ATE', 'DEMO-COB'))
    OR servicio_cargo_id::text LIKE '00000000-0000-4000-8000-00000000d0%'
    OR servicio_cargo_id::text LIKE '00000000-0000-4000-8000-00000000e0%';
DELETE FROM servicio_cargos
 WHERE servicio_id IN (SELECT id FROM servicios WHERE codigo IN ('DEMO-ATE', 'DEMO-COB'))
    OR id::text LIKE '00000000-0000-4000-8000-00000000e0%';
DELETE FROM usuario_servicios WHERE servicio_id IN (SELECT id FROM servicios WHERE codigo IN ('DEMO-ATE', 'DEMO-COB'))
    OR servicio_id = '00000000-0000-4000-8000-00000000e031';
DELETE FROM servicios WHERE codigo IN ('DEMO-ATE', 'DEMO-COB') OR id = '00000000-0000-4000-8000-00000000e031';
DELETE FROM cargos WHERE nombre IN ('Agente Demo', 'Asistente Demo', 'Supervisor Demo Compartido')
    OR id::text LIKE '00000000-0000-4000-8000-00000000e0%';
DELETE FROM contratos_comerciales WHERE codigo = 'K-DEMO';
DELETE FROM clientes WHERE nombre = 'Cliente Demo S.A.';
DELETE FROM centros_costo WHERE codigo IN ('DEMO-A', 'DEMO-B');
DELETE FROM feriados WHERE nombre = 'Feriado de demostración (ficticio)';
COMMIT;

-- -------------------------------------------------------------------------------------
-- 1. Catálogo (INSERT directo)
-- -------------------------------------------------------------------------------------
BEGIN;
INSERT INTO clientes (id, nombre) VALUES
  ('00000000-0000-4000-8000-00000000d001', 'Cliente Demo S.A.');

INSERT INTO centros_costo (id, codigo, nombre) VALUES
  ('00000000-0000-4000-8000-00000000d011', 'DEMO-A', 'Centro Demo Atención'),
  ('00000000-0000-4000-8000-00000000d012', 'DEMO-B', 'Centro Demo Cobranza');

-- Vigente hoy → los servicios cuentan como "configurados" en el Tablero.
INSERT INTO contratos_comerciales (id, codigo, nombre, cliente_id, fecha_inicio, fecha_fin) VALUES
  ('00000000-0000-4000-8000-00000000d021', 'K-DEMO', 'Contrato Demo',
   '00000000-0000-4000-8000-00000000d001', '2026-01-01', '2027-12-31');

INSERT INTO servicios (id, codigo, nombre, centro_costo_id, contrato_comercial_id, cliente_id,
                       tipo_contrato, modalidad, tipo_turno, dias_trabajo, departamento_region,
                       tiempo_capacitacion_dias, capacitacion_pagada, tiene_refrigerio, activo) VALUES
  ('00000000-0000-4000-8000-00000000d031', 'DEMO-ATE', 'Demo Atención',
   '00000000-0000-4000-8000-00000000d011', '00000000-0000-4000-8000-00000000d021',
   '00000000-0000-4000-8000-00000000d001', 'Indefinido', 'Presencial', 'turno',
   ARRAY['Lunes','Martes','Miércoles','Jueves','Viernes'], 'Santa Cruz', 7, true, false, true),
  ('00000000-0000-4000-8000-00000000d032', 'DEMO-COB', 'Demo Cobranza',
   '00000000-0000-4000-8000-00000000d012', '00000000-0000-4000-8000-00000000d021',
   '00000000-0000-4000-8000-00000000d001', 'Indefinido', 'Presencial', 'continuo',
   ARRAY['Lunes','Martes','Miércoles','Jueves','Viernes','Sábado'], 'La Paz', 5, true, true, true);

INSERT INTO cargos (id, nombre, tipo) VALUES
  ('00000000-0000-4000-8000-00000000d041', 'Agente Demo', 'operativo'),
  ('00000000-0000-4000-8000-00000000d042', 'Asistente Demo', 'operativo'),
  ('00000000-0000-4000-8000-00000000d043', 'Supervisor Demo Compartido', 'operativo');

-- Agente Demo: los 3 tiempos propios (10 / 5 / 12 días). Solo en Demo Atención.
-- Asistente Demo: sin tiempos propios (usa los 7 días generales de Demo Atención).
-- Supervisor Demo Compartido: en los dos servicios (dos centros de costo distintos).
INSERT INTO servicio_cargos (id, servicio_id, cargo_id, haber_basico_diurno, haber_basico_nocturno,
                             cantidad_mujeres, cantidad_hombres,
                             tiempo_capacitacion_dias, tiempo_ojt_dias, tiempo_capacitacion_ajustada_dias) VALUES
  ('00000000-0000-4000-8000-00000000d051', '00000000-0000-4000-8000-00000000d031',
   '00000000-0000-4000-8000-00000000d041', 3800, 400, 6, 4, 10, 5, 12),
  ('00000000-0000-4000-8000-00000000d052', '00000000-0000-4000-8000-00000000d031',
   '00000000-0000-4000-8000-00000000d042', 3700, NULL, 2, 1, NULL, NULL, NULL),
  ('00000000-0000-4000-8000-00000000d053', '00000000-0000-4000-8000-00000000d031',
   '00000000-0000-4000-8000-00000000d043', 5000, NULL, 1, 1, NULL, NULL, NULL),
  ('00000000-0000-4000-8000-00000000d054', '00000000-0000-4000-8000-00000000d032',
   '00000000-0000-4000-8000-00000000d043', 4800, NULL, 1, 0, NULL, NULL, NULL);

-- Un bono fijo para Agente Demo (se congela en los requerimientos de ese cargo).
INSERT INTO servicio_cargo_metricas (servicio_cargo_id, metrica_id, peso_pct, monto_bono)
SELECT '00000000-0000-4000-8000-00000000d051', m.id, 100, 300
  FROM catalogo_metricas m WHERE m.nombre = 'Calidad' LIMIT 1;

-- Reparto del cargo compartido: 4.200 + 2.800 = 7.000 → 60 % / 40 %.
INSERT INTO cargo_centros_costo (cargo_id, centro_costo_id, monto) VALUES
  ('00000000-0000-4000-8000-00000000d043', '00000000-0000-4000-8000-00000000d011', 4200),
  ('00000000-0000-4000-8000-00000000d043', '00000000-0000-4000-8000-00000000d012', 2800);

-- Feriado ficticio (viernes 20/11/2026) para que se note en los días hábiles de reclutamiento.
INSERT INTO feriados (fecha, nombre) VALUES ('2026-11-20', 'Feriado de demostración (ficticio)')
ON CONFLICT (fecha) DO NOTHING;
COMMIT;

-- -------------------------------------------------------------------------------------
-- 2. Requerimientos y ediciones: con las funciones oficiales, como el usuario RRHH
-- -------------------------------------------------------------------------------------
-- Cada paso es un bloque anónimo que se identifica como RRHH igual que el backend en cada
-- request (db::rls: rol `authenticated` + claims del JWT, locales a la transacción).
-- La fecha de capacitación que se envía es la de operación (como la pantalla "Nuevo
-- requerimiento"); la base la recalcula si corresponde.
\set COMO_RRHH 'PERFORM set_config(''request.jwt.claims'', ''{"sub":"00000000-0000-0000-0000-aaaa00000001","role":"authenticated"}'', true); PERFORM set_config(''request.jwt.claim.sub'', ''00000000-0000-0000-0000-aaaa00000001'', true); PERFORM set_config(''request.jwt.claim.role'', ''authenticated'', true); SET LOCAL ROLE authenticated;'

CREATE TEMP TABLE demo_pasos (orden int, sql text);
INSERT INTO demo_pasos VALUES
-- Caso 1 (Nuevo · Pendiente): cantidad que se aumenta dos veces.
(1, $q$PERFORM crear_requerimiento_con_snapshot(jsonb_build_object('tipo','nuevo',
     'servicioId','00000000-0000-4000-8000-00000000d031','cargoId','00000000-0000-4000-8000-00000000d041',
     'cantidadPersonas',5,'motivo','Crecimiento','fechaInicioCapacitacion','2026-12-15',
     'fechaInicioOperacion','2026-12-15','comentarios','[DEMO 1] Cantidad aumentada con glosa y desvío','bonosExtra','[]'::jsonb))$q$),
-- Caso 2 (Modificación · Aprobado): fecha de operación que se mueve dos veces.
(2, $q$PERFORM crear_requerimiento_con_snapshot(jsonb_build_object('tipo','modificacion',
     'servicioId','00000000-0000-4000-8000-00000000d031','cargoId','00000000-0000-4000-8000-00000000d042',
     'cantidadPersonas',3,'motivo','Reposición','fechaInicioCapacitacion','2026-12-01',
     'fechaInicioOperacion','2026-12-01','comentarios','[DEMO 2] Fecha de operación modificada con glosa y desvío','bonosExtra','[]'::jsonb))$q$),
-- Caso 3 (Cambio · En proceso): con entrega y rectificación (Almanaque "quedan N").
(3, $q$PERFORM crear_requerimiento_con_snapshot(jsonb_build_object('tipo','cambio',
     'servicioId','00000000-0000-4000-8000-00000000d031','cargoId','00000000-0000-4000-8000-00000000d041',
     'cantidadPersonas',6,'motivo','Renuncia','fechaInicioCapacitacion','2026-12-01',
     'fechaInicioOperacion','2026-12-01','comentarios','[DEMO 3] Entrega de 3 rectificada con +2','bonosExtra','[]'::jsonb))$q$),
-- Caso 4 (Promoción · Rechazado): cargo compartido, en Demo Cobranza.
(4, $q$PERFORM crear_requerimiento_con_snapshot(jsonb_build_object('tipo','promocion',
     'servicioId','00000000-0000-4000-8000-00000000d032','cargoId','00000000-0000-4000-8000-00000000d043',
     'cantidadPersonas',1,'motivo','Crecimiento','fechaInicioCapacitacion','2026-12-07',
     'fechaInicioOperacion','2026-12-07','comentarios','[DEMO 4] Promoción interna rechazada','bonosExtra','[]'::jsonb))$q$),
-- Caso 5 (Nuevo · Pendiente): el que se crea desde un día vacío del Almanaque (26/11/2026).
(5, $q$PERFORM crear_requerimiento_con_snapshot(jsonb_build_object('tipo','nuevo',
     'servicioId','00000000-0000-4000-8000-00000000d032','cargoId','00000000-0000-4000-8000-00000000d043',
     'cantidadPersonas',2,'motivo','Nuevo proyecto','fechaInicioCapacitacion','2026-11-26',
     'fechaInicioOperacion','2026-11-26','comentarios','[DEMO 5] Creado desde el día vacío 26/11/2026 del Almanaque','bonosExtra','[]'::jsonb))$q$);

-- Ediciones con trazabilidad: (marca, cambios, glosa, categoría de desvío, detalle).
CREATE TEMP TABLE demo_ediciones (orden int, marca text, cambios jsonb, glosa text, categoria text, detalle text);
INSERT INTO demo_ediciones VALUES
(1, '[DEMO 1]', '{"cantidad_personas": 7}', 'El cliente amplió la campaña en dos puestos',
    'cambio_cliente', 'Ampliación de campaña de fin de año'),
(2, '[DEMO 1]', '{"cantidad_personas": 8}', 'Se suma un puesto de respaldo para cubrir ausencias', NULL, NULL),
(3, '[DEMO 2]', '{"fecha_inicio_operacion": "2026-12-10"}', 'El cliente postergó el inicio de la campaña',
    'cambio_cliente', 'Nueva fecha de arranque informada por el cliente'),
(4, '[DEMO 2]', '{"fecha_inicio_operacion": "2026-12-12"}', 'Ajuste por disponibilidad de la sala de capacitación', NULL, NULL),
-- Estados (solo RR.HH. puede cambiarlos; también quedan en el historial).
(5, '[DEMO 2]', '{"estado": "aprobado"}', 'Aprobado por RR.HH.', NULL, NULL),
(6, '[DEMO 3]', '{"estado": "en_proceso"}', 'Reclutamiento en curso', NULL, NULL),
(7, '[DEMO 4]', '{"estado": "rechazado"}', 'El postulante no cumple la antigüedad mínima para la promoción', NULL, NULL);
GRANT SELECT ON demo_pasos, demo_ediciones TO authenticated;

-- psql no tiene bucles: un \gexec genera un bloque DO por paso, cada uno en su transacción
-- (autocommit), para que el historial quede con horas distintas.
SELECT format('DO $do$ BEGIN %s %s; END $do$', :'COMO_RRHH', sql) FROM demo_pasos ORDER BY orden \gexec

SELECT format($f$DO $do$ DECLARE v_id uuid; v_ver timestamptz; BEGIN %s
  SELECT id, updated_at INTO STRICT v_id, v_ver FROM requerimientos WHERE comentarios_adicionales LIKE %L;
  PERFORM editar_requerimiento_con_historial(v_id, %L::jsonb, v_ver, %L, %L, %L);
END $do$$f$, :'COMO_RRHH', marca || '%', cambios, glosa, categoria, detalle)
  FROM demo_ediciones ORDER BY orden \gexec

-- -------------------------------------------------------------------------------------
-- 4. Entrega + rectificación del caso 3 (INSERT directo; ver encabezado)
-- -------------------------------------------------------------------------------------
-- Reglas de producción respetadas (p2_validar_entrega / CHECKs):
--   * la entrega tiene cantidad >= 0; la rectificación cantidad <> 0 y cantidad_reclutada NULL;
--   * una rectificación solo corrige una entrega de tipo 'entrega', ACEPTADA, del mismo requerimiento.
BEGIN;
INSERT INTO requerimiento_entregas (id, requerimiento_id, cargo_id, fecha_entrega, cantidad, tipo, glosa, registrado_por)
SELECT '00000000-0000-4000-8000-00000000d061', r.id, r.cargo_id, '2026-11-18', 3, 'entrega',
       'Primer grupo entregado a capacitación', '00000000-0000-0000-0000-aaaa00000001'
  FROM requerimientos r WHERE r.comentarios_adicionales LIKE '[DEMO 3]%';
INSERT INTO requerimiento_entrega_decisiones (entrega_id, estado, comentario, decidido_por)
VALUES ('00000000-0000-4000-8000-00000000d061', 'aceptada', 'Recibidos por capacitación',
        '00000000-0000-0000-0000-aaaa00000001');
COMMIT;
BEGIN;
INSERT INTO requerimiento_entregas (id, requerimiento_id, cargo_id, fecha_entrega, cantidad, tipo,
                                    corrige_entrega_id, glosa, registrado_por)
SELECT '00000000-0000-4000-8000-00000000d062', r.id, r.cargo_id, '2026-11-19', 2, 'rectificacion',
       '00000000-0000-4000-8000-00000000d061', 'Se sumaron dos personas al grupo del 18/11',
       '00000000-0000-0000-0000-aaaa00000001'
  FROM requerimientos r WHERE r.comentarios_adicionales LIKE '[DEMO 3]%';
COMMIT;

-- -------------------------------------------------------------------------------------
-- Resumen
-- -------------------------------------------------------------------------------------
SELECT r.folio, r.tipo, r.estado, s.nombre AS servicio, c.nombre AS cargo, r.cantidad_personas AS cantidad,
       to_char(r.fecha_inicio_capacitacion, 'DD/MM/YYYY') AS capacitacion,
       to_char(r.fecha_inicio_operacion, 'DD/MM/YYYY') AS operacion,
       left(r.comentarios_adicionales, 9) AS caso
  FROM requerimientos r JOIN servicios s ON s.id = r.servicio_id JOIN cargos c ON c.id = r.cargo_id
 WHERE r.comentarios_adicionales LIKE '[DEMO %' ORDER BY r.comentarios_adicionales;

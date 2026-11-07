-- 0006: la fecha de inicio de capacitación que guarda la base al crear un requerimiento usa el
-- tiempo de capacitación del CARGO en el servicio (`servicio_cargos.tiempo_capacitacion_dias`)
-- y, si el cargo no lo tiene, el tiempo general del servicio (`servicios.tiempo_capacitacion_dias`).
-- Mismo criterio que la "fecha de inicio máximo de capacitación" de la pantalla de nuevo
-- requerimiento (domain::plazos). Decisión del usuario del 2026-11-07.
--
-- NO ES ADITIVA: reemplaza (CREATE OR REPLACE) la función de trigger de producción
-- `validar_fechas_nuevo_requerimiento`, que ya está en uso. Se mantienen las condiciones
-- (cargo operativo y tipo distinto de promoción), el mensaje de error y el snapshot
-- (`tiempo_capacitacion_snap` = días efectivamente usados). Cambia de dónde salen los días:
--   - si el cargo tiene su tiempo cargado (aunque sea 0), capacitación = operación − ese tiempo;
--   - si no, como antes: operación − tiempo general del servicio, solo si es > 0.
-- La función `crear_requerimiento_con_snapshot` NO se toca: sigue calculando con el tiempo del
-- servicio, pero este trigger (BEFORE INSERT) corre después y deja el valor final.
-- No toca filas existentes: solo afecta requerimientos que se creen después.
--
-- Producción: NO aplicada. Requiere confirmación explícita, respaldo y ensayo con ROLLBACK.
--
-- Para volver atrás, la definición original (pg_get_functiondef, igual a producción):
--   CREATE OR REPLACE FUNCTION public.validar_fechas_nuevo_requerimiento()
--    RETURNS trigger LANGUAGE plpgsql SET search_path TO 'public'
--   AS $function$
--   DECLARE v_tipo text; v_dias integer;
--   BEGIN
--    SELECT tipo INTO v_tipo FROM public.cargos WHERE id=NEW.cargo_id;
--    SELECT tiempo_capacitacion_dias INTO v_dias FROM public.servicios WHERE id=NEW.servicio_id;
--    NEW.tiempo_capacitacion_snap:=v_dias;
--    IF v_tipo='operativo' AND NEW.tipo<>'promocion' AND v_dias>0 THEN
--      NEW.fecha_inicio_capacitacion:=NEW.fecha_inicio_operacion-v_dias;
--    END IF;
--    IF NEW.fecha_inicio_capacitacion IS NULL OR NEW.fecha_inicio_operacion IS NULL THEN RAISE EXCEPTION 'Indica la fecha de inicio de capacitación y la fecha de inicio de operación.'; END IF;
--    RETURN NEW;
--   END $function$;

CREATE OR REPLACE FUNCTION public.validar_fechas_nuevo_requerimiento()
 RETURNS trigger
 LANGUAGE plpgsql
 SET search_path TO 'public'
AS $function$
DECLARE v_tipo text; v_dias integer; v_dias_cargo integer;
BEGIN
 SELECT tipo INTO v_tipo FROM public.cargos WHERE id=NEW.cargo_id;
 -- Tiempo del cargo en el servicio; si no está cargado, el general del servicio.
 SELECT sc.tiempo_capacitacion_dias INTO v_dias_cargo FROM public.servicio_cargos sc
  WHERE sc.servicio_id=NEW.servicio_id AND sc.cargo_id=NEW.cargo_id;
 IF v_dias_cargo IS NOT NULL THEN
   v_dias:=v_dias_cargo;
 ELSE
   SELECT tiempo_capacitacion_dias INTO v_dias FROM public.servicios WHERE id=NEW.servicio_id;
 END IF;
 NEW.tiempo_capacitacion_snap:=v_dias;
 IF v_tipo='operativo' AND NEW.tipo<>'promocion'
    AND ((v_dias_cargo IS NOT NULL AND v_dias_cargo>=0) OR v_dias>0) THEN
   NEW.fecha_inicio_capacitacion:=NEW.fecha_inicio_operacion-v_dias;
 END IF;
 IF NEW.fecha_inicio_capacitacion IS NULL OR NEW.fecha_inicio_operacion IS NULL THEN RAISE EXCEPTION 'Indica la fecha de inicio de capacitación y la fecha de inicio de operación.'; END IF;
 RETURN NEW;
END $function$;

-- Aviso visible en la base para quien lea `crear_requerimiento_con_snapshot` (solo metadato:
-- no cambia su código ni su comportamiento).
COMMENT ON FUNCTION public.crear_requerimiento_con_snapshot(jsonb) IS
  'El cálculo de fecha_inicio_capacitacion de esta función (con el tiempo del servicio) queda '
  'sobreescrito por el trigger validar_fechas_nuevo_requerimiento (BEFORE INSERT), que usa el '
  'tiempo del cargo en el servicio y, si no lo tiene, el del servicio. La fecha real se define '
  'en el trigger (migración 0006).';

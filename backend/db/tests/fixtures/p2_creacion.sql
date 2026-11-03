-- SOLO PARA TESTS. Copia TEXTUAL (pg_get_functiondef, 2026-11-03) de la creación de
-- requerimientos en producción: función `crear_requerimiento_con_snapshot` y trigger
-- `validar_fechas_nuevo_requerimiento`. Requiere `p2_edicion` (requerimiento_referencia,
-- trigger de auditoría). Si cambian en producción, actualizar esta copia.

CREATE OR REPLACE FUNCTION public.validar_fechas_nuevo_requerimiento()
 RETURNS trigger
 LANGUAGE plpgsql
 SET search_path TO 'public'
AS $function$
DECLARE v_tipo text; v_dias integer;
BEGIN
 SELECT tipo INTO v_tipo FROM public.cargos WHERE id=NEW.cargo_id;
 SELECT tiempo_capacitacion_dias INTO v_dias FROM public.servicios WHERE id=NEW.servicio_id;
 NEW.tiempo_capacitacion_snap:=v_dias;
 IF v_tipo='operativo' AND NEW.tipo<>'promocion' AND v_dias>0 THEN
   NEW.fecha_inicio_capacitacion:=NEW.fecha_inicio_operacion-v_dias;
 END IF;
 IF NEW.fecha_inicio_capacitacion IS NULL OR NEW.fecha_inicio_operacion IS NULL THEN RAISE EXCEPTION 'Indica la fecha de inicio de capacitación y la fecha de inicio de operación.'; END IF;
 RETURN NEW;
END $function$
;
CREATE OR REPLACE FUNCTION public.crear_requerimiento_con_snapshot(p_requerimiento jsonb)
 RETURNS TABLE(folio text)
 LANGUAGE plpgsql
 SECURITY DEFINER
 SET search_path TO 'public'
AS $function$
DECLARE
  v_actor uuid := auth.uid();
  v_rol text;
  v_servicio uuid;
  v_cargo uuid;
  v_tipo_cargo text;
  v_inicio date;
  v_fecha_capacitacion date;
  v_cantidad integer;
  v_sc public.servicio_cargos%ROWTYPE;
  v_capacitacion integer := NULL;
  v_cap_input text;
  v_reclutador uuid := NULL;
  v_bonos jsonb := '[]'::jsonb;
  v_extras jsonb;
  v_extra jsonb;
BEGIN
  SELECT u.rol INTO v_rol FROM public.usuarios u WHERE u.id = v_actor AND u.activo;
  IF v_actor IS NULL OR v_rol IS NULL THEN RAISE EXCEPTION 'Debes iniciar sesión con una cuenta activa.'; END IF;
  IF v_rol = 'externo' THEN RAISE EXCEPTION 'Tu rol no permite crear requerimientos.'; END IF;
  IF p_requerimiento IS NULL OR jsonb_typeof(p_requerimiento) IS DISTINCT FROM 'object' THEN RAISE EXCEPTION 'Datos inválidos.'; END IF;
  IF p_requerimiento - ARRAY['tipo','servicioId','cargoId','cantidadPersonas','motivo','fechaInicioCapacitacion','fechaInicioOperacion','comentarios','bonosExtra','tiempoCapacitacionDias','fechaFin'] <> '{}'::jsonb THEN RAISE EXCEPTION 'Campo no permitido.'; END IF;
  IF coalesce(p_requerimiento->>'tipo','') NOT IN ('nuevo','modificacion','promocion','cambio') THEN RAISE EXCEPTION 'Elige el tipo de requerimiento.'; END IF;
  IF jsonb_typeof(p_requerimiento->'cantidadPersonas') IS DISTINCT FROM 'number' OR coalesce(p_requerimiento->>'cantidadPersonas','') !~ '^[1-9][0-9]*$' THEN RAISE EXCEPTION 'Indica al menos 1 persona.'; END IF;
  v_cantidad := (p_requerimiento->>'cantidadPersonas')::integer;
  IF coalesce(p_requerimiento->>'motivo','') NOT IN ('Renuncia','Baja médica','Nuevo proyecto','Reposición','Crecimiento') THEN RAISE EXCEPTION 'Elige un motivo.'; END IF;
  IF jsonb_typeof(p_requerimiento->'fechaInicioCapacitacion') IS DISTINCT FROM 'string' OR coalesce(p_requerimiento->>'fechaInicioCapacitacion','') !~ '^[0-9]{4}-[0-9]{2}-[0-9]{2}$' THEN RAISE EXCEPTION 'Indica la fecha de inicio de capacitación.'; END IF;
  IF jsonb_typeof(p_requerimiento->'fechaInicioOperacion') IS DISTINCT FROM 'string' OR coalesce(p_requerimiento->>'fechaInicioOperacion','') !~ '^[0-9]{4}-[0-9]{2}-[0-9]{2}$' THEN RAISE EXCEPTION 'Indica la fecha de inicio de operación.'; END IF;
  v_fecha_capacitacion := (p_requerimiento->>'fechaInicioCapacitacion')::date;
  v_inicio := (p_requerimiento->>'fechaInicioOperacion')::date;
  IF to_char(v_fecha_capacitacion,'YYYY-MM-DD') IS DISTINCT FROM p_requerimiento->>'fechaInicioCapacitacion' OR to_char(v_inicio,'YYYY-MM-DD') IS DISTINCT FROM p_requerimiento->>'fechaInicioOperacion' THEN RAISE EXCEPTION 'Fecha inválida.'; END IF;
  IF p_requerimiento ? 'comentarios' AND jsonb_typeof(p_requerimiento->'comentarios') NOT IN ('string','null') THEN RAISE EXCEPTION 'Comentarios inválidos.'; END IF;
  v_servicio := nullif(p_requerimiento->>'servicioId','')::uuid;
  v_cargo := nullif(p_requerimiento->>'cargoId','')::uuid;
  SELECT c.tipo INTO v_tipo_cargo FROM public.cargos c WHERE c.id = v_cargo;
  IF NOT FOUND THEN RAISE EXCEPTION 'Elige un cargo válido.'; END IF;
  IF v_servicio IS NULL THEN
    IF v_tipo_cargo <> 'administrativo_staff' THEN RAISE EXCEPTION 'Elige un servicio para ese cargo.'; END IF;
  ELSE
    SELECT s.tiempo_capacitacion_dias INTO v_capacitacion FROM public.servicios s
    WHERE s.id = v_servicio AND s.activo AND (v_rol = 'rrhh_admin' OR EXISTS (SELECT 1 FROM public.usuario_servicios us WHERE us.usuario_id = v_actor AND us.servicio_id = s.id));
    IF NOT FOUND THEN RAISE EXCEPTION 'No tienes acceso a ese servicio activo.'; END IF;
  END IF;
  IF p_requerimiento->>'fechaFin' IS NOT NULL THEN
    IF coalesce(p_requerimiento->>'fechaFin','') !~ '^[0-9]{4}-[0-9]{2}-[0-9]{2}$' OR to_char((p_requerimiento->>'fechaFin')::date,'YYYY-MM-DD') IS DISTINCT FROM p_requerimiento->>'fechaFin' THEN RAISE EXCEPTION 'Fecha fin inválida.'; END IF;
  END IF;
  -- Creation duration belongs to the selected service, never to the caller.
  IF p_requerimiento->>'tiempoCapacitacionDias' IS NOT NULL THEN
    RAISE EXCEPTION 'La duración de capacitación procede del servicio y no se edita al crear.';
  END IF;
  IF v_tipo_cargo = 'operativo' AND p_requerimiento->>'tipo' <> 'promocion' AND v_capacitacion > 0 THEN
    v_fecha_capacitacion := v_inicio - v_capacitacion;
  END IF;
  BEGIN
    SELECT sc.* INTO STRICT v_sc FROM public.servicio_cargos sc WHERE sc.cargo_id = v_cargo AND sc.servicio_id IS NOT DISTINCT FROM v_servicio;
  EXCEPTION
    WHEN no_data_found THEN IF v_servicio IS NOT NULL THEN RAISE EXCEPTION 'Ese cargo no pertenece al servicio elegido.'; END IF;
    WHEN too_many_rows THEN RAISE EXCEPTION 'Hay más de una composición para ese cargo.';
  END;
  SELECT coalesce(jsonb_agg(jsonb_build_object('metrica',cm.nombre,'monto',scm.monto_bono) ORDER BY cm.nombre,scm.id),'[]'::jsonb) INTO v_bonos
  FROM public.servicio_cargo_metricas scm JOIN public.catalogo_metricas cm ON cm.id = scm.metrica_id WHERE scm.servicio_cargo_id = v_sc.id;
  v_extras := coalesce(p_requerimiento->'bonosExtra','[]'::jsonb);
  IF jsonb_typeof(v_extras) IS DISTINCT FROM 'array' THEN RAISE EXCEPTION 'Bonos extra inválidos.'; END IF;
  FOR v_extra IN SELECT value FROM jsonb_array_elements(v_extras) LOOP
    IF jsonb_typeof(v_extra) IS DISTINCT FROM 'object' THEN RAISE EXCEPTION 'Bono extra inválido.'; END IF;
    IF v_extra - ARRAY['tipo','nombre','monto','motivo'] <> '{}'::jsonb OR v_extra->>'tipo' IS DISTINCT FROM 'extra' OR jsonb_typeof(v_extra->'nombre') IS DISTINCT FROM 'string' OR length(btrim(v_extra->>'nombre')) = 0 OR jsonb_typeof(v_extra->'motivo') IS DISTINCT FROM 'string' OR length(btrim(v_extra->>'motivo')) = 0 OR jsonb_typeof(v_extra->'monto') IS DISTINCT FROM 'number' THEN RAISE EXCEPTION 'Indica nombre, monto y motivo del bono extra.'; END IF;
    IF (v_extra->>'monto')::numeric <= 0 OR round((v_extra->>'monto')::numeric,2) <> (v_extra->>'monto')::numeric THEN RAISE EXCEPTION 'El monto debe ser positivo y tener hasta dos decimales.'; END IF;
    v_bonos := v_bonos || jsonb_build_array(jsonb_build_object('tipo','extra','nombre',btrim(v_extra->>'nombre'),'monto',(v_extra->>'monto')::numeric,'motivo',btrim(v_extra->>'motivo')));
  END LOOP;
  IF v_tipo_cargo = 'administrativo_staff' THEN
    SELECT u.id INTO v_reclutador FROM public.usuarios u WHERE u.es_reclutador_staff_default AND u.activo ORDER BY u.created_at,u.id LIMIT 1;
  END IF;
  RETURN QUERY INSERT INTO public.requerimientos AS r (
    solicitante_id,servicio_id,cargo_id,tipo,cantidad_personas,motivo,fecha_fin,fecha_inicio,fecha_fin_tentativa,fecha_inicio_capacitacion,fecha_inicio_operacion,comentarios_adicionales,haber_basico_diurno_snap,haber_basico_nocturno_snap,bonos_snap,tiempo_capacitacion_snap,reclutador_asignado_id,estado
  ) VALUES (
    v_actor,v_servicio,v_cargo,p_requerimiento->>'tipo',v_cantidad,p_requerimiento->>'motivo',(p_requerimiento->>'fechaFin')::date,v_inicio,NULL,v_fecha_capacitacion,v_inicio,nullif(btrim(p_requerimiento->>'comentarios'),''),v_sc.haber_basico_diurno,v_sc.haber_basico_nocturno,v_bonos,v_capacitacion,v_reclutador,'pendiente'
  ) RETURNING coalesce(r.folio,'#' || r.numero::text);
END;
$function$
;

CREATE TRIGGER requerimientos_validar_fechas BEFORE INSERT ON public.requerimientos FOR EACH ROW EXECUTE FUNCTION validar_fechas_nuevo_requerimiento();
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO authenticated;

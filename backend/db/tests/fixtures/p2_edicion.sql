-- SOLO PARA TESTS. Copia TEXTUAL de las funciones, triggers y políticas de producción que
-- gobiernan la edición de `requerimientos` (leídas de la base real con pg_get_functiondef /
-- pg_policy, 2026-11-03). Se carga DESPUÉS de `datos` para que los triggers de INSERT no
-- afecten la carga de datos de prueba. Reemplaza las políticas simplificadas de UPDATE de
-- requerimientos e INSERT de historial de `rls_test.sql`.
-- Si cambian en producción, actualizar esta copia.

CREATE TABLE requerimiento_referencia (
    requerimiento_id uuid NOT NULL REFERENCES requerimientos(id),
    cantidad_original integer,
    capacitacion_original date,
    operacion_original date,
    origen text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz
);
GRANT SELECT, INSERT, UPDATE ON requerimiento_referencia TO authenticated;

CREATE OR REPLACE FUNCTION public.current_user_role()
 RETURNS text LANGUAGE sql STABLE SECURITY DEFINER SET search_path TO 'public'
AS $function$
  select rol from usuarios where id = auth.uid();
$function$;

CREATE OR REPLACE FUNCTION public.is_rrhh_admin(uid uuid)
 RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER SET search_path TO 'public'
AS $function$
  select exists (
    select 1 from usuarios where id = uid and rol = 'rrhh_admin'
  );
$function$;

-- En producción estas funciones no son SECURITY DEFINER y leen `usuarios` con las políticas
-- reales. La política de prueba `t_usuarios` solo deja ver la propia fila (o todo a RR.HH.),
-- suficiente para estas funciones excepto el caso "inmediato superior" (no cubierto aquí).
CREATE OR REPLACE FUNCTION public.usuario_activo_actual()
 RETURNS boolean LANGUAGE sql STABLE SET search_path TO 'public'
AS $function$
  SELECT EXISTS (SELECT 1 FROM public.usuarios u WHERE u.id = auth.uid() AND u.activo);
$function$;

CREATE OR REPLACE FUNCTION public.tiene_servicio_asignado(p_servicio_id uuid)
 RETURNS boolean LANGUAGE sql STABLE SET search_path TO 'public'
AS $function$
  SELECT p_servicio_id IS NOT NULL AND EXISTS (
    SELECT 1
    FROM public.usuario_servicios us
    JOIN public.usuarios u ON u.id = us.usuario_id
    WHERE us.usuario_id = auth.uid()
      AND us.servicio_id = p_servicio_id
      AND u.activo
      AND u.rol <> 'externo');
$function$;

CREATE OR REPLACE FUNCTION public.p2_acceso(p_id uuid)
 RETURNS boolean LANGUAGE sql STABLE SET search_path TO 'public'
AS $function$
 SELECT public.usuario_activo_actual() AND public.current_user_role()<>'externo'
 AND EXISTS (SELECT 1 FROM public.requerimientos r WHERE r.id=p_id AND
 (public.is_rrhh_admin(auth.uid()) OR r.solicitante_id=auth.uid()
 OR EXISTS (SELECT 1 FROM public.usuarios solicitante WHERE solicitante.id=r.solicitante_id AND solicitante.inmediato_superior_id=auth.uid())
 OR (public.current_user_role()='estandar' AND public.tiene_servicio_asignado(r.servicio_id))));
$function$;

CREATE OR REPLACE FUNCTION public.p2_validar_desvio(c text, d text)
 RETURNS void LANGUAGE plpgsql SET search_path TO 'public'
AS $function$
BEGIN
 IF c IS NOT NULL AND c NOT IN ('falla_capacitacion','rechazo_operaciones','ratio_reclutamiento','cambio_cliente') THEN
  RAISE EXCEPTION 'Categoría de desvío inválida';
 END IF;
 IF (c IS NULL AND nullif(btrim(d),'') IS NOT NULL)
 OR (c IS NOT NULL AND length(btrim(coalesce(d,'')))<3)
 OR length(coalesce(d,''))>2000 THEN
  RAISE EXCEPTION 'Indicar categoría y explicación juntas (3 a 2000 caracteres)';
 END IF;
END $function$;

CREATE OR REPLACE FUNCTION public.p2_validar_campos()
 RETURNS trigger LANGUAGE plpgsql SET search_path TO 'public'
AS $function$
DECLARE x jsonb; prev jsonb; actual jsonb; i integer;
BEGIN
 IF (to_jsonb(NEW)-ARRAY['folio','updated_at','cantidad_personas','fecha_fin','fecha_salida','fecha_inicio_capacitacion','fecha_inicio_operacion','motivo','comentarios_adicionales','estado','tiempo_capacitacion_snap','bonos_snap'])
 IS DISTINCT FROM (to_jsonb(OLD)-ARRAY['folio','updated_at','cantidad_personas','fecha_fin','fecha_salida','fecha_inicio_capacitacion','fecha_inicio_operacion','motivo','comentarios_adicionales','estado','tiempo_capacitacion_snap','bonos_snap']) THEN RAISE EXCEPTION 'Campo fuera del alcance de edición'; END IF;
 IF NEW.motivo IS DISTINCT FROM OLD.motivo AND coalesce(NEW.motivo,'') NOT IN ('Renuncia','Baja médica','Nuevo proyecto','Reposición','Crecimiento') THEN RAISE EXCEPTION 'Motivo inválido'; END IF;
 IF NEW.estado IS DISTINCT FROM OLD.estado AND coalesce(NEW.estado,'') NOT IN ('pendiente','en_proceso','aprobado','rechazado','cerrado') THEN RAISE EXCEPTION 'Estado inválido'; END IF;
 IF NEW.tiempo_capacitacion_snap IS DISTINCT FROM OLD.tiempo_capacitacion_snap AND (NEW.tiempo_capacitacion_snap IS NULL OR NEW.tiempo_capacitacion_snap NOT BETWEEN 1 AND 99999) THEN RAISE EXCEPTION 'Duración de capacitación inválida'; END IF;
 IF NEW.bonos_snap IS DISTINCT FROM OLD.bonos_snap THEN
  prev:=CASE WHEN OLD.bonos_snap IS NULL OR OLD.bonos_snap='null'::jsonb THEN '[]'::jsonb ELSE OLD.bonos_snap END;
  actual:=NEW.bonos_snap;
  IF jsonb_typeof(prev)<>'array' OR jsonb_typeof(actual) IS DISTINCT FROM 'array' OR jsonb_array_length(actual)<=jsonb_array_length(prev) THEN RAISE EXCEPTION 'Solo añadir bonos extra'; END IF;
  IF jsonb_array_length(prev)>0 THEN
   FOR i IN 0..jsonb_array_length(prev)-1 LOOP
    IF actual->i IS DISTINCT FROM prev->i THEN RAISE EXCEPTION 'Bonos existentes inmutables'; END IF;
   END LOOP;
  END IF;
  FOR i IN jsonb_array_length(prev)..jsonb_array_length(actual)-1 LOOP
   x:=actual->i;
   IF jsonb_typeof(x)<>'object' OR x-ARRAY['tipo','nombre','monto','motivo']<>'{}'::jsonb
   OR x->>'tipo' IS DISTINCT FROM 'extra' OR jsonb_typeof(x->'nombre') IS DISTINCT FROM 'string'
   OR jsonb_typeof(x->'motivo') IS DISTINCT FROM 'string' OR length(btrim(x->>'nombre'))=0
   OR length(btrim(x->>'motivo'))=0 OR jsonb_typeof(x->'monto') IS DISTINCT FROM 'number' THEN RAISE EXCEPTION 'Bono extra inválido'; END IF;
   IF (x->>'monto')::numeric<=0 OR round((x->>'monto')::numeric,2)<>(x->>'monto')::numeric THEN RAISE EXCEPTION 'Monto del bono inválido'; END IF;
  END LOOP;
 END IF;
 RETURN NEW;
END $function$;

CREATE OR REPLACE FUNCTION public.proteger_campos_requerimiento()
 RETURNS trigger LANGUAGE plpgsql SET search_path TO 'public'
AS $function$
DECLARE admin boolean; g text; total bigint;
BEGIN
 IF NOT public.p2_acceso(OLD.id) THEN RAISE EXCEPTION 'Sin acceso activo al requerimiento'; END IF;
 admin:=public.is_rrhh_admin(auth.uid());
 g:=nullif(btrim(current_setting('app.p2_glosa',true)),'');
 IF g IS NULL OR length(g) NOT BETWEEN 3 AND 2000 THEN RAISE EXCEPTION 'Glosa obligatoria (3 a 2000 caracteres)'; END IF;
 IF NOT admin AND NEW.fecha_inicio_capacitacion IS DISTINCT FROM OLD.fecha_inicio_capacitacion THEN RAISE EXCEPTION 'La fecha de capacitación es exclusiva de RR.HH.'; END IF;
 IF NOT admin AND NEW.fecha_salida IS DISTINCT FROM OLD.fecha_salida THEN RAISE EXCEPTION 'Fecha de salida exclusiva de RR.HH.'; END IF;
 IF NOT admin THEN
  IF (to_jsonb(NEW)-ARRAY['folio','cantidad_personas','fecha_fin','fecha_inicio_capacitacion','fecha_inicio_operacion','motivo','comentarios_adicionales','updated_at'])
  IS DISTINCT FROM (to_jsonb(OLD)-ARRAY['folio','cantidad_personas','fecha_fin','fecha_inicio_capacitacion','fecha_inicio_operacion','motivo','comentarios_adicionales','updated_at']) THEN
   RAISE EXCEPTION 'Campo no autorizado';
  END IF;
  IF OLD.estado<>'pendiente' AND (NEW.motivo,NEW.comentarios_adicionales) IS DISTINCT FROM (OLD.motivo,OLD.comentarios_adicionales) THEN
   RAISE EXCEPTION 'Motivo y comentarios solo se editan en pendiente';
  END IF;
 END IF;
 IF (NEW.id,NEW.numero,NEW.solicitante_id,NEW.servicio_id,NEW.cargo_id,NEW.tipo,NEW.fecha_inicio,NEW.fecha_fin_tentativa,NEW.haber_basico_diurno_snap,NEW.haber_basico_nocturno_snap,NEW.created_at)
 IS DISTINCT FROM (OLD.id,OLD.numero,OLD.solicitante_id,OLD.servicio_id,OLD.cargo_id,OLD.tipo,OLD.fecha_inicio,OLD.fecha_fin_tentativa,OLD.haber_basico_diurno_snap,OLD.haber_basico_nocturno_snap,OLD.created_at) THEN RAISE EXCEPTION 'Datos originales protegidos'; END IF;
 IF NEW.cantidad_personas IS DISTINCT FROM OLD.cantidad_personas THEN
  SELECT coalesce(sum(e.cantidad),0) INTO total FROM public.requerimiento_entregas e JOIN public.requerimiento_entrega_decisiones d ON d.entrega_id=e.id AND d.estado='aceptada' WHERE e.requerimiento_id=OLD.id;
  IF NEW.cantidad_personas IS NULL OR NEW.cantidad_personas<1 OR NEW.cantidad_personas<total THEN RAISE EXCEPTION 'Cantidad inválida o inferior a lo ya aceptado'; END IF;
 END IF;
 IF NEW.fecha_inicio_capacitacion IS DISTINCT FROM OLD.fecha_inicio_capacitacion AND NEW.fecha_inicio_capacitacion IS NULL
 OR NEW.fecha_inicio_operacion IS DISTINCT FROM OLD.fecha_inicio_operacion AND NEW.fecha_inicio_operacion IS NULL THEN RAISE EXCEPTION 'No se puede borrar una fecha'; END IF;
 IF (to_jsonb(NEW)-ARRAY['folio','updated_at']) IS NOT DISTINCT FROM (to_jsonb(OLD)-ARRAY['folio','updated_at']) THEN RAISE EXCEPTION 'No hay cambios para guardar'; END IF;
 NEW.updated_at:=clock_timestamp(); RETURN NEW;
END $function$;

CREATE OR REPLACE FUNCTION public.p2_auditar_requerimiento()
 RETURNS trigger LANGUAGE plpgsql SET search_path TO 'public'
AS $function$
DECLARE k text; grupo uuid:=gen_random_uuid(); c text; d text;
BEGIN
 IF TG_OP='INSERT' THEN
  INSERT INTO public.requerimiento_referencia(requerimiento_id,cantidad_original,capacitacion_original,operacion_original,origen)
  VALUES(NEW.id,NEW.cantidad_personas,NEW.fecha_inicio_capacitacion,NEW.fecha_inicio_operacion,'creacion');
 ELSE
  c:=nullif(current_setting('app.p2_categoria',true),''); d:=nullif(current_setting('app.p2_detalle',true),'');
  PERFORM public.p2_validar_desvio(c,d);
  FOR k IN SELECT jsonb_object_keys(to_jsonb(NEW)-ARRAY['folio','updated_at']) LOOP
   IF (to_jsonb(NEW)->k) IS DISTINCT FROM (to_jsonb(OLD)->k) THEN
    INSERT INTO public.requerimiento_historial(requerimiento_id,campo,valor_anterior,valor_nuevo,usuario_id,glosa,grupo_edicion,categoria_desvio,detalle_desvio)
    VALUES(NEW.id,k,to_jsonb(OLD)->>k,to_jsonb(NEW)->>k,auth.uid(),current_setting('app.p2_glosa',true),grupo,c,d);
   END IF;
  END LOOP;
 END IF;
 RETURN NEW;
END $function$;

CREATE OR REPLACE FUNCTION public.editar_requerimiento_con_historial(p_id uuid, p_cambios jsonb, p_version timestamp with time zone, p_glosa text, p_categoria_desvio text DEFAULT NULL::text, p_detalle_desvio text DEFAULT NULL::text)
 RETURNS uuid LANGUAGE plpgsql SET search_path TO 'public'
AS $function$
DECLARE r public.requerimientos%ROWTYPE; n public.requerimientos%ROWTYPE; k text; b jsonb; x jsonb; admin boolean;
BEGIN
 IF NOT public.p2_acceso(p_id) THEN RAISE EXCEPTION 'Sin acceso al requerimiento'; END IF;
 SELECT * INTO r FROM public.requerimientos WHERE id=p_id FOR UPDATE;
 IF r.updated_at IS DISTINCT FROM p_version THEN RAISE EXCEPTION 'El requerimiento cambió; actualizar antes de guardar'; END IF;
 IF jsonb_typeof(p_cambios) IS DISTINCT FROM 'object' THEN RAISE EXCEPTION 'Cambios inválidos'; END IF;
 IF length(btrim(coalesce(p_glosa,''))) NOT BETWEEN 3 AND 2000 THEN RAISE EXCEPTION 'Glosa obligatoria (3 a 2000 caracteres)'; END IF;
 PERFORM public.p2_validar_desvio(p_categoria_desvio,p_detalle_desvio);
 admin:=public.is_rrhh_admin(auth.uid());
 FOR k IN SELECT jsonb_object_keys(p_cambios) LOOP
  IF k NOT IN ('cantidad_personas','fecha_fin','fecha_salida','fecha_inicio_capacitacion','fecha_inicio_operacion','motivo','comentarios_adicionales','estado','tiempo_capacitacion_snap','bonos_extra','fecha_inicio_capacitacion','fecha_salida') THEN RAISE EXCEPTION 'Campo no editable'; END IF;
  IF NOT admin AND k IN ('estado','tiempo_capacitacion_snap','bonos_extra','fecha_inicio_capacitacion','fecha_salida') THEN RAISE EXCEPTION 'Campo exclusivo de RR.HH.'; END IF;
  IF NOT admin AND r.estado<>'pendiente' AND k IN ('motivo','comentarios_adicionales') THEN RAISE EXCEPTION 'Campo editable solo en pendiente'; END IF;
  IF k IN ('cantidad_personas','tiempo_capacitacion_snap') AND coalesce(p_cambios->>k,'') !~ '^[1-9][0-9]*$' THEN RAISE EXCEPTION 'Cantidad o duración inválida'; END IF;
  IF k IN ('fecha_fin','fecha_salida') AND p_cambios->>k IS NOT NULL THEN
   IF coalesce(p_cambios->>k,'') !~ '^[0-9]{4}-[0-9]{2}-[0-9]{2}$' OR to_char((p_cambios->>k)::date,'YYYY-MM-DD')<>p_cambios->>k THEN RAISE EXCEPTION 'Fecha fin inválida'; END IF;
  END IF;
  IF k IN ('fecha_inicio_capacitacion','fecha_inicio_operacion') THEN
   IF coalesce(p_cambios->>k,'') !~ '^[0-9]{4}-[0-9]{2}-[0-9]{2}$'
   OR to_char((p_cambios->>k)::date,'YYYY-MM-DD')<>p_cambios->>k THEN RAISE EXCEPTION 'Fecha inválida'; END IF;
  END IF;
 END LOOP;
 SELECT * INTO n FROM jsonb_populate_record(r,p_cambios-'bonos_extra');
 b:=coalesce(r.bonos_snap,'[]'::jsonb);
 IF p_cambios ? 'bonos_extra' THEN
  IF jsonb_typeof(p_cambios->'bonos_extra') IS DISTINCT FROM 'array' OR jsonb_typeof(b)<>'array' THEN RAISE EXCEPTION 'Bonos inválidos'; END IF;
  FOR x IN SELECT value FROM jsonb_array_elements(p_cambios->'bonos_extra') LOOP
   b:=b||jsonb_build_array(x);
  END LOOP;
 END IF;
 PERFORM set_config('app.p2_glosa',btrim(p_glosa),true);
 PERFORM set_config('app.p2_categoria',coalesce(p_categoria_desvio,''),true);
 PERFORM set_config('app.p2_detalle',coalesce(btrim(p_detalle_desvio),''),true);
 UPDATE public.requerimientos SET cantidad_personas=n.cantidad_personas,fecha_fin=n.fecha_fin,fecha_salida=n.fecha_salida,
 fecha_inicio_capacitacion=n.fecha_inicio_capacitacion,fecha_inicio_operacion=n.fecha_inicio_operacion,
 motivo=n.motivo,comentarios_adicionales=n.comentarios_adicionales,estado=n.estado,
 tiempo_capacitacion_snap=n.tiempo_capacitacion_snap,
 bonos_snap=CASE WHEN p_cambios ? 'bonos_extra' THEN b ELSE r.bonos_snap END WHERE id=p_id;
 PERFORM set_config('app.p2_glosa','',true); PERFORM set_config('app.p2_categoria','',true); PERFORM set_config('app.p2_detalle','',true);
 RETURN p_id;
END $function$;

CREATE TRIGGER requerimientos_p2_auditar AFTER INSERT OR UPDATE ON public.requerimientos FOR EACH ROW EXECUTE FUNCTION p2_auditar_requerimiento();
CREATE TRIGGER requerimientos_p2_validar BEFORE UPDATE ON public.requerimientos FOR EACH ROW EXECUTE FUNCTION p2_validar_campos();
CREATE TRIGGER requerimientos_proteger_campos BEFORE UPDATE ON public.requerimientos FOR EACH ROW EXECUTE FUNCTION proteger_campos_requerimiento();

GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO authenticated;

-- Políticas reales (reemplazan las simplificadas de rls_test.sql).
DROP POLICY t_req_upd ON requerimientos;
DROP POLICY t_hist_ins ON requerimiento_historial;
CREATE POLICY requerimientos_update ON requerimientos FOR UPDATE
    USING (p2_acceso(id)) WITH CHECK (p2_acceso(id));
CREATE POLICY requerimiento_historial_insert ON requerimiento_historial FOR INSERT
    WITH CHECK (EXISTS ( SELECT 1
   FROM requerimientos r
  WHERE ((r.id = requerimiento_historial.requerimiento_id) AND (is_rrhh_admin(auth.uid()) OR (r.solicitante_id = auth.uid()) OR tiene_servicio_asignado(r.servicio_id) OR (usuario_activo_actual() AND (current_user_role() <> 'externo'::text) AND (EXISTS ( SELECT 1
           FROM usuarios solicitante
          WHERE ((solicitante.id = r.solicitante_id) AND (solicitante.inmediato_superior_id = auth.uid())))))))));

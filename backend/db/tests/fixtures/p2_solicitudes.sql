-- SOLO PARA TESTS. Copia TEXTUAL (2026-11-03) de las reglas de producción de
-- `solicitudes_cambio`: CHECKs, trigger `validar_solicitud_cambio`, función `es_costos_activo`
-- y políticas RLS. Requiere `p2_edicion` (is_rrhh_admin, usuario_activo_actual,
-- tiene_servicio_asignado). Reemplaza la política simplificada `t_sol` de rls_test.sql.

ALTER TABLE solicitudes_cambio
  ADD CONSTRAINT solicitudes_cambio_campo_check CHECK ((campo = ANY (ARRAY['centro_costo'::text, 'cliente'::text, 'contrato_comercial'::text, 'tipo_contrato'::text, 'modalidad'::text, 'dias_trabajo'::text, 'sueldo_basico'::text, 'bonos'::text]))),
  ADD CONSTRAINT solicitudes_cambio_comentario_resolucion_check CHECK (((comentario_resolucion IS NULL) OR (length(comentario_resolucion) <= 2000))),
  ADD CONSTRAINT solicitudes_cambio_descripcion_check CHECK (((descripcion IS NULL) OR (length(descripcion) <= 2000))),
  ADD CONSTRAINT solicitudes_cambio_estado_check CHECK ((estado = ANY (ARRAY['pendiente'::text, 'resuelta'::text, 'rechazada'::text, 'cancelada'::text]))),
  ADD CONSTRAINT solicitudes_cambio_motivo_check CHECK (((length(btrim(motivo)) >= 3) AND (length(btrim(motivo)) <= 500))),
  ADD CONSTRAINT solicitudes_cargo_requerido CHECK (((campo <> ALL (ARRAY['sueldo_basico'::text, 'bonos'::text])) OR (cargo_id IS NOT NULL))),
  ADD CONSTRAINT solicitudes_cierre_coherente CHECK ((((estado = 'pendiente'::text) AND (cerrada_at IS NULL) AND (responsable_id IS NULL) AND (comentario_resolucion IS NULL)) OR ((estado = 'cancelada'::text) AND (cerrada_at IS NOT NULL)) OR ((estado = ANY (ARRAY['resuelta'::text, 'rechazada'::text])) AND (cerrada_at IS NOT NULL) AND (responsable_id IS NOT NULL) AND (length(btrim(COALESCE(comentario_resolucion, ''::text))) >= 3))));

CREATE OR REPLACE FUNCTION public.es_costos_activo()
 RETURNS boolean
 LANGUAGE sql
 STABLE
 SET search_path TO 'public'
AS $function$
  SELECT EXISTS (SELECT 1 FROM public.usuarios u
                 WHERE u.id = auth.uid() AND u.activo AND u.rol = 'encargado_costos');
$function$;

CREATE OR REPLACE FUNCTION public.validar_solicitud_cambio()
 RETURNS trigger
 LANGUAGE plpgsql
 SET search_path TO 'public'
AS $function$
DECLARE
  v_responsable boolean;
BEGIN
  IF TG_OP = 'INSERT' THEN
    NEW.motivo := btrim(NEW.motivo);
    NEW.descripcion := nullif(btrim(NEW.descripcion), '');
    NEW.estado := 'pendiente';
    NEW.responsable_id := NULL;
    NEW.comentario_resolucion := NULL;
    NEW.cerrada_at := NULL;
    NEW.created_at := now();
    NEW.updated_at := now();
    IF NEW.cargo_id IS NOT NULL AND NOT EXISTS (
      SELECT 1 FROM public.cargos c WHERE c.id = NEW.cargo_id
    ) THEN
      RAISE EXCEPTION 'El cargo indicado no existe';
    END IF;
    RETURN NEW;
  END IF;

  -- UPDATE: lo original no se modifica
  NEW.id := OLD.id;
  NEW.servicio_id := OLD.servicio_id;
  NEW.cargo_id := OLD.cargo_id;
  NEW.campo := OLD.campo;
  NEW.solicitante_id := OLD.solicitante_id;
  NEW.motivo := OLD.motivo;
  NEW.descripcion := OLD.descripcion;
  NEW.created_at := OLD.created_at;

  IF OLD.estado <> 'pendiente' THEN
    RAISE EXCEPTION 'La solicitud ya está cerrada';
  END IF;

  v_responsable := public.is_rrhh_admin(auth.uid())
                   OR (OLD.campo = 'centro_costo' AND public.es_costos_activo());

  IF NEW.estado = 'cancelada' THEN
    IF OLD.solicitante_id <> auth.uid() THEN
      RAISE EXCEPTION 'Solo quien pidió el cambio puede cancelarlo';
    END IF;
    NEW.responsable_id := NULL;
    NEW.comentario_resolucion := nullif(btrim(NEW.comentario_resolucion), '');
  ELSIF NEW.estado IN ('resuelta','rechazada') THEN
    IF NOT v_responsable THEN
      RAISE EXCEPTION 'No tienes permiso para cerrar esta solicitud';
    END IF;
    NEW.responsable_id := auth.uid();
    NEW.comentario_resolucion := btrim(coalesce(NEW.comentario_resolucion, ''));
    IF length(NEW.comentario_resolucion) < 3 THEN
      RAISE EXCEPTION 'El comentario de resolución es obligatorio';
    END IF;
  ELSE
    RAISE EXCEPTION 'Transición de estado no permitida';
  END IF;

  NEW.cerrada_at := now();
  NEW.updated_at := now();
  RETURN NEW;
END $function$;

CREATE TRIGGER validar_solicitud_cambio BEFORE INSERT OR UPDATE ON public.solicitudes_cambio
    FOR EACH ROW EXECUTE FUNCTION validar_solicitud_cambio();
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO authenticated;

DROP POLICY t_sol ON solicitudes_cambio;
CREATE POLICY solicitudes_cambio_insert ON solicitudes_cambio FOR INSERT
    WITH CHECK ((usuario_activo_actual() AND (solicitante_id = auth.uid()) AND (estado = 'pendiente'::text) AND (is_rrhh_admin(auth.uid()) OR tiene_servicio_asignado(servicio_id))));
CREATE POLICY solicitudes_cambio_select ON solicitudes_cambio FOR SELECT
    USING ((usuario_activo_actual() AND ((solicitante_id = auth.uid()) OR is_rrhh_admin(auth.uid()) OR ((campo = 'centro_costo'::text) AND es_costos_activo()))));
CREATE POLICY solicitudes_cambio_update ON solicitudes_cambio FOR UPDATE
    USING ((usuario_activo_actual() AND (estado = 'pendiente'::text) AND ((solicitante_id = auth.uid()) OR is_rrhh_admin(auth.uid()) OR ((campo = 'centro_costo'::text) AND es_costos_activo()))))
    WITH CHECK ((usuario_activo_actual() AND ((solicitante_id = auth.uid()) OR is_rrhh_admin(auth.uid()) OR ((campo = 'centro_costo'::text) AND es_costos_activo()))));

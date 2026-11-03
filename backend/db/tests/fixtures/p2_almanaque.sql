-- SOLO PARA TESTS. Copia TEXTUAL (pg_get_functiondef, 2026-11-03) de la función de
-- producción que alimenta el almanaque. Si cambia en producción, actualizar esta copia.

CREATE OR REPLACE FUNCTION public.leer_eventos_almanaque(p_desde date, p_hasta date)
 RETURNS TABLE(tipo_evento text, fecha date, cliente text, servicio text, cargo text, cantidad integer, folio text, estado_entrega text)
 LANGUAGE plpgsql
 STABLE SECURITY DEFINER
 SET search_path TO 'public'
AS $function$
DECLARE v_usuario uuid:=auth.uid();v_rol text;
BEGIN
 SELECT u.rol INTO v_rol FROM public.usuarios u WHERE u.id=v_usuario AND u.activo;
 IF v_usuario IS NULL OR v_rol IS NULL OR v_rol NOT IN ('rrhh_admin','encargado_costos','estandar') THEN RAISE EXCEPTION 'No autorizado' USING ERRCODE='42501'; END IF;
 IF p_desde IS NULL OR p_hasta IS NULL OR p_hasta<p_desde OR p_hasta-p_desde>93 THEN RAISE EXCEPTION 'Rango de fechas inválido' USING ERRCODE='22023'; END IF;
 RETURN QUERY WITH accesibles AS (
 SELECT r.id,r.servicio_id,r.cargo_id,r.fecha_inicio_operacion,r.fecha_salida,r.cantidad_personas,r.folio FROM public.requerimientos r
 WHERE v_rol='rrhh_admin' OR EXISTS(SELECT 1 FROM public.usuario_servicios us WHERE us.usuario_id=v_usuario AND us.servicio_id=r.servicio_id)
 ), eventos AS (
 SELECT 'in'::text tipo_evento,r.fecha_inicio_operacion fecha,c.nombre cliente,s.nombre servicio,ca.nombre cargo,r.cantidad_personas cantidad,r.folio,NULL::text estado_entrega
 FROM accesibles r LEFT JOIN public.servicios s ON s.id=r.servicio_id LEFT JOIN public.clientes c ON c.id=s.cliente_id JOIN public.cargos ca ON ca.id=r.cargo_id WHERE r.fecha_inicio_operacion BETWEEN p_desde AND p_hasta
 UNION ALL
 SELECT 'out',r.fecha_salida,c.nombre,s.nombre,ca.nombre,r.cantidad_personas,r.folio,NULL::text
 FROM accesibles r LEFT JOIN public.servicios s ON s.id=r.servicio_id LEFT JOIN public.clientes c ON c.id=s.cliente_id JOIN public.cargos ca ON ca.id=r.cargo_id WHERE r.fecha_salida BETWEEN p_desde AND p_hasta
 UNION ALL
 SELECT e.tipo,e.fecha_entrega,c.nombre,s.nombre,ca.nombre,e.cantidad,r.folio,coalesce(d.estado,'pendiente')
 FROM public.requerimiento_entregas e JOIN accesibles r ON r.id=e.requerimiento_id LEFT JOIN public.servicios s ON s.id=r.servicio_id LEFT JOIN public.clientes c ON c.id=s.cliente_id JOIN public.cargos ca ON ca.id=e.cargo_id LEFT JOIN public.requerimiento_entrega_decisiones d ON d.entrega_id=e.id WHERE e.fecha_entrega BETWEEN p_desde AND p_hasta
 ) SELECT ev.tipo_evento,ev.fecha,ev.cliente,ev.servicio,ev.cargo,ev.cantidad,ev.folio,ev.estado_entrega FROM eventos ev ORDER BY ev.fecha,ev.folio,ev.tipo_evento,ev.cargo;
END $function$
;
GRANT EXECUTE ON FUNCTION public.leer_eventos_almanaque(date, date) TO authenticated;

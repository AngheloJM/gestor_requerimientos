-- SOLO PARA TESTS. Reglas de producción (leídas el 2026-11-03) de las tablas de configuración
-- de la Matriz de Servicio: restricciones, triggers (copia textual) y políticas RLS (copia
-- textual, exportadas con pg_policy). Requiere `p2_edicion` (is_rrhh_admin,
-- usuario_activo_actual). Ajusta el esquema aproximado de `supabase_subset` a producción.

-- Esquema: diferencias con producción ------------------------------------------------------
UPDATE cargos SET tipo = 'operativo' WHERE tipo IS NULL;
ALTER TABLE cargos ALTER COLUMN tipo SET NOT NULL,
  ADD CONSTRAINT cargos_tipo_check CHECK ((tipo = ANY (ARRAY['operativo'::text, 'administrativo_staff'::text]))),
  ADD CONSTRAINT cargos_nombre_tipo_key UNIQUE (nombre, tipo);

UPDATE servicios SET modalidad = 'presencial' WHERE modalidad IS NULL;
UPDATE servicios SET dias_trabajo = '{}' WHERE dias_trabajo IS NULL;
ALTER TABLE servicios
  ALTER COLUMN modalidad SET DEFAULT 'presencial', ALTER COLUMN modalidad SET NOT NULL,
  ALTER COLUMN dias_trabajo SET DEFAULT '{}'::text[], ALTER COLUMN dias_trabajo SET NOT NULL,
  ALTER COLUMN capacitacion_pagada SET DEFAULT false, ALTER COLUMN tiene_refrigerio SET DEFAULT false,
  ADD CONSTRAINT servicios_modalidad_check CHECK ((modalidad = ANY (ARRAY['Presencial'::text, 'Remoto'::text, 'Híbrido'::text, 'presencial'::text, 'remoto'::text, 'híbrido'::text]))),
  ADD CONSTRAINT servicios_tipo_turno_check CHECK ((tipo_turno = ANY (ARRAY['continuo'::text, 'turno'::text])));

ALTER TABLE contratos_comerciales ALTER COLUMN nombre DROP NOT NULL, ALTER COLUMN fecha_inicio DROP NOT NULL,
  DROP CONSTRAINT IF EXISTS contratos_comerciales_check;

ALTER TABLE proyectos ADD CONSTRAINT proyectos_cliente_id_nombre_key UNIQUE (cliente_id, nombre);
ALTER TABLE servicio_cargos ADD CONSTRAINT servicio_cargos_servicio_id_cargo_id_key UNIQUE (servicio_id, cargo_id);
ALTER TABLE servicio_cargo_metricas
  DROP CONSTRAINT servicio_cargo_metricas_servicio_cargo_id_fkey,
  ADD CONSTRAINT servicio_cargo_metricas_servicio_cargo_id_fkey FOREIGN KEY (servicio_cargo_id) REFERENCES servicio_cargos(id) ON DELETE CASCADE,
  ADD CONSTRAINT servicio_cargo_metricas_servicio_cargo_id_metrica_id_key UNIQUE (servicio_cargo_id, metrica_id);
ALTER TABLE servicio_cargo_metrica_umbrales ALTER COLUMN orden SET DEFAULT 0,
  DROP CONSTRAINT servicio_cargo_metrica_umbrales_servicio_cargo_metrica_id_fkey,
  ADD CONSTRAINT servicio_cargo_metrica_umbrales_servicio_cargo_metrica_id_fkey FOREIGN KEY (servicio_cargo_metrica_id) REFERENCES servicio_cargo_metricas(id) ON DELETE CASCADE;

CREATE TABLE catalogo_bonos (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    nombre text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE modalidades (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    nombre text NOT NULL CHECK ((length(btrim(nombre)) > 0)),
    activo boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE tipos_contrato (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    nombre text NOT NULL CHECK ((length(btrim(nombre)) > 0)),
    activo boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
-- Valores reales de producción (catálogos genéricos, no son datos de clientes).
INSERT INTO modalidades (nombre) VALUES ('Presencial'), ('Remoto'), ('Híbrido');
INSERT INTO tipos_contrato (nombre) VALUES ('Plazo Fijo'), ('Indefinido'), ('Eventual'), ('Consultoría por Producto');

-- Triggers (copia textual) -----------------------------------------------------------------
CREATE OR REPLACE FUNCTION public.validar_vigencia_contrato_comercial()
 RETURNS trigger LANGUAGE plpgsql SET search_path TO 'public'
AS $function$
BEGIN
  IF NEW.fecha_inicio IS NULL
     AND NEW.fecha_fin IS NULL
     AND NEW.duracion_valor IS NULL
     AND NEW.duracion_unidad IS NULL THEN
    -- Solo se conserva vacío el legado que ya estaba vacío.
    IF TG_OP = 'UPDATE' THEN
      IF OLD.fecha_inicio IS NULL
         AND OLD.fecha_fin IS NULL
         AND OLD.duracion_valor IS NULL
         AND OLD.duracion_unidad IS NULL THEN
        RETURN NEW;
      END IF;
    END IF;
    RAISE EXCEPTION 'Fecha de inicio y vigencia obligatorias para contratos nuevos o completados.';
  END IF;

  IF NEW.fecha_inicio IS NULL THEN
    RAISE EXCEPTION 'Fecha de inicio obligatoria.';
  END IF;

  IF NEW.fecha_fin IS NOT NULL THEN
    IF NEW.duracion_valor IS NOT NULL
       OR NEW.duracion_unidad IS NOT NULL THEN
      RAISE EXCEPTION 'Elige fecha de fin o duración, no ambas.';
    END IF;
    IF NEW.fecha_fin < NEW.fecha_inicio THEN
      RAISE EXCEPTION 'La fecha de fin no puede ser anterior al inicio.';
    END IF;
  ELSE
    IF NEW.duracion_valor IS NULL
       OR NEW.duracion_valor <= 0 THEN
      RAISE EXCEPTION 'La duración debe ser un entero positivo.';
    END IF;
    IF NEW.duracion_unidad IS NULL
       OR NEW.duracion_unidad NOT IN ('meses', 'dias') THEN
      RAISE EXCEPTION 'La unidad de duración debe ser meses o días.';
    END IF;
  END IF;

  RETURN NEW;
END;
$function$;
CREATE TRIGGER validar_vigencia_contrato_comercial BEFORE INSERT OR UPDATE ON public.contratos_comerciales FOR EACH ROW EXECUTE FUNCTION validar_vigencia_contrato_comercial();

CREATE OR REPLACE FUNCTION public.validar_catalogo_bonos()
 RETURNS trigger LANGUAGE plpgsql SET search_path TO 'public'
AS $function$
BEGIN
  NEW.nombre := btrim(
    regexp_replace(NEW.nombre, '[[:space:]]+', ' ', 'g')
  );

  IF NEW.nombre IS NULL OR NEW.nombre = '' THEN
    RAISE EXCEPTION 'El nombre del bono es obligatorio.'
      USING ERRCODE = '23514';
  END IF;

  IF TG_OP = 'INSERT' THEN
    NEW.created_at := now();
  ELSE
    NEW.created_at := OLD.created_at;
  END IF;

  NEW.updated_at := now();
  RETURN NEW;
END;
$function$;
CREATE TRIGGER validar_catalogo_bonos BEFORE INSERT OR UPDATE ON public.catalogo_bonos FOR EACH ROW EXECUTE FUNCTION validar_catalogo_bonos();

CREATE OR REPLACE FUNCTION public.validar_proyecto_servicio()
 RETURNS trigger LANGUAGE plpgsql SET search_path TO 'public'
AS $function$
BEGIN
  NEW.updated_at := now();
  IF NEW.nombre IS NULL OR btrim(NEW.nombre) = '' THEN RAISE EXCEPTION 'Nombre de proyecto obligatorio.'; END IF;
  NEW.nombre := btrim(NEW.nombre);
  IF TG_OP = 'UPDATE' AND NEW.cliente_id IS DISTINCT FROM OLD.cliente_id
     AND EXISTS (SELECT 1 FROM servicios WHERE proyecto_id = NEW.id) THEN
    RAISE EXCEPTION 'No se puede cambiar el cliente de un proyecto con servicios.';
  END IF;
  RETURN NEW;
END $function$;
CREATE TRIGGER proyectos_validar BEFORE INSERT OR UPDATE ON public.proyectos FOR EACH ROW EXECUTE FUNCTION validar_proyecto_servicio();

CREATE OR REPLACE FUNCTION public.validar_servicio_proyecto()
 RETURNS trigger LANGUAGE plpgsql SET search_path TO 'public'
AS $function$
BEGIN
  IF NEW.proyecto_id IS NOT NULL AND NOT EXISTS (
    SELECT 1 FROM proyectos p WHERE p.id = NEW.proyecto_id AND p.cliente_id = NEW.cliente_id) THEN
    RAISE EXCEPTION 'El proyecto no corresponde al cliente del servicio.';
  END IF;
  RETURN NEW;
END $function$;
CREATE TRIGGER servicios_validar_proyecto BEFORE INSERT OR UPDATE OF proyecto_id, cliente_id ON public.servicios FOR EACH ROW EXECUTE FUNCTION validar_servicio_proyecto();

CREATE OR REPLACE FUNCTION public.puede_ver_servicio(p_servicio_id uuid)
 RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER SET search_path TO 'public'
AS $function$
  SELECT EXISTS (SELECT 1 FROM usuarios u WHERE u.id = auth.uid() AND u.activo
    AND (u.rol = 'rrhh_admin' OR (u.rol IN ('encargado_costos','estandar')
      AND EXISTS (SELECT 1 FROM usuario_servicios us WHERE us.usuario_id = u.id AND us.servicio_id = p_servicio_id))));
$function$;

-- RLS y políticas (copia textual de producción) ----------------------------------------------
GRANT SELECT, INSERT, UPDATE, DELETE ON clientes, proyectos, centros_costo, servicios, cargos, servicio_cargos,
  contratos_comerciales, servicio_cargo_metricas, servicio_cargo_metrica_umbrales, catalogo_metricas,
  catalogo_bonos, modalidades, tipos_contrato TO authenticated;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO authenticated;
DROP POLICY t_servicios ON servicios;
DROP POLICY t_sc ON servicio_cargos;
ALTER TABLE clientes ENABLE ROW LEVEL SECURITY;
ALTER TABLE proyectos ENABLE ROW LEVEL SECURITY;
ALTER TABLE centros_costo ENABLE ROW LEVEL SECURITY;
ALTER TABLE cargos ENABLE ROW LEVEL SECURITY;
ALTER TABLE contratos_comerciales ENABLE ROW LEVEL SECURITY;
ALTER TABLE servicio_cargo_metricas ENABLE ROW LEVEL SECURITY;
ALTER TABLE servicio_cargo_metrica_umbrales ENABLE ROW LEVEL SECURITY;
ALTER TABLE catalogo_metricas ENABLE ROW LEVEL SECURITY;
ALTER TABLE catalogo_bonos ENABLE ROW LEVEL SECURITY;
ALTER TABLE modalidades ENABLE ROW LEVEL SECURITY;
ALTER TABLE tipos_contrato ENABLE ROW LEVEL SECURITY;
CREATE POLICY cargos_select ON cargos FOR SELECT USING (true);
CREATE POLICY cargos_write ON cargos FOR ALL USING (is_rrhh_admin(auth.uid())) WITH CHECK (is_rrhh_admin(auth.uid()));
CREATE POLICY catalogo_bonos_alta ON catalogo_bonos FOR INSERT WITH CHECK ((is_rrhh_admin(auth.uid()) AND (EXISTS ( SELECT 1
   FROM usuarios u
  WHERE ((u.id = auth.uid()) AND (u.activo = true))))));
CREATE POLICY catalogo_bonos_edicion ON catalogo_bonos FOR UPDATE USING ((is_rrhh_admin(auth.uid()) AND (EXISTS ( SELECT 1
   FROM usuarios u
  WHERE ((u.id = auth.uid()) AND (u.activo = true)))))) WITH CHECK ((is_rrhh_admin(auth.uid()) AND (EXISTS ( SELECT 1
   FROM usuarios u
  WHERE ((u.id = auth.uid()) AND (u.activo = true))))));
CREATE POLICY catalogo_bonos_lectura ON catalogo_bonos FOR SELECT USING ((EXISTS ( SELECT 1
   FROM usuarios u
  WHERE ((u.id = auth.uid()) AND (u.activo = true) AND (u.rol = ANY (ARRAY['rrhh_admin'::text, 'encargado_costos'::text]))))));
CREATE POLICY catalogo_insert ON catalogo_metricas FOR INSERT WITH CHECK (is_rrhh_admin(auth.uid()));
CREATE POLICY catalogo_select ON catalogo_metricas FOR SELECT USING (true);
CREATE POLICY centros_costo_select ON centros_costo FOR SELECT USING ((is_rrhh_admin(auth.uid()) OR (EXISTS ( SELECT 1
   FROM servicios s
  WHERE (s.centro_costo_id = centros_costo.id)))));
CREATE POLICY centros_costo_write ON centros_costo FOR ALL USING (is_rrhh_admin(auth.uid())) WITH CHECK (is_rrhh_admin(auth.uid()));
CREATE POLICY clientes_select ON clientes FOR SELECT USING ((is_rrhh_admin(auth.uid()) OR (EXISTS ( SELECT 1
   FROM servicios s
  WHERE (s.cliente_id = clientes.id)))));
CREATE POLICY clientes_write ON clientes FOR ALL USING (is_rrhh_admin(auth.uid())) WITH CHECK (is_rrhh_admin(auth.uid()));
CREATE POLICY contratos_select ON contratos_comerciales FOR SELECT USING ((is_rrhh_admin(auth.uid()) OR (EXISTS ( SELECT 1
   FROM servicios s
  WHERE (s.contrato_comercial_id = contratos_comerciales.id)))));
CREATE POLICY contratos_write ON contratos_comerciales FOR ALL USING (is_rrhh_admin(auth.uid())) WITH CHECK (is_rrhh_admin(auth.uid()));
CREATE POLICY modalidades_insert ON modalidades FOR INSERT WITH CHECK (is_rrhh_admin(auth.uid()));
CREATE POLICY modalidades_select ON modalidades FOR SELECT USING (true);
CREATE POLICY modalidades_update ON modalidades FOR UPDATE USING (is_rrhh_admin(auth.uid())) WITH CHECK (is_rrhh_admin(auth.uid()));
CREATE POLICY proyectos_insert ON proyectos FOR INSERT WITH CHECK ((is_rrhh_admin(auth.uid()) AND usuario_activo_actual()));
CREATE POLICY proyectos_select ON proyectos FOR SELECT USING ((is_rrhh_admin(auth.uid()) OR (EXISTS ( SELECT 1
   FROM servicios s
  WHERE (s.proyecto_id = proyectos.id)))));
CREATE POLICY proyectos_update ON proyectos FOR UPDATE USING ((is_rrhh_admin(auth.uid()) AND usuario_activo_actual())) WITH CHECK ((is_rrhh_admin(auth.uid()) AND usuario_activo_actual()));
CREATE POLICY umbrales_select ON servicio_cargo_metrica_umbrales FOR SELECT USING ((EXISTS ( SELECT 1
   FROM servicio_cargo_metricas m
  WHERE (m.id = servicio_cargo_metrica_umbrales.servicio_cargo_metrica_id))));
CREATE POLICY umbrales_write ON servicio_cargo_metrica_umbrales FOR ALL USING (is_rrhh_admin(auth.uid())) WITH CHECK (is_rrhh_admin(auth.uid()));
CREATE POLICY scm_select ON servicio_cargo_metricas FOR SELECT USING ((EXISTS ( SELECT 1
   FROM servicio_cargos sc
  WHERE (sc.id = servicio_cargo_metricas.servicio_cargo_id))));
CREATE POLICY scm_write ON servicio_cargo_metricas FOR ALL USING (is_rrhh_admin(auth.uid())) WITH CHECK (is_rrhh_admin(auth.uid()));
CREATE POLICY servicio_cargos_select ON servicio_cargos FOR SELECT USING ((is_rrhh_admin(auth.uid()) OR ((servicio_id IS NOT NULL) AND puede_ver_servicio(servicio_id))));
CREATE POLICY servicio_cargos_write ON servicio_cargos FOR ALL USING (is_rrhh_admin(auth.uid())) WITH CHECK (is_rrhh_admin(auth.uid()));
CREATE POLICY servicios_select ON servicios FOR SELECT USING ((puede_ver_servicio(id) OR (EXISTS ( SELECT 1
   FROM requerimientos r
  WHERE (r.servicio_id = servicios.id)))));
CREATE POLICY servicios_write ON servicios FOR ALL USING (is_rrhh_admin(auth.uid())) WITH CHECK (is_rrhh_admin(auth.uid()));
CREATE POLICY tipos_contrato_insert ON tipos_contrato FOR INSERT WITH CHECK (is_rrhh_admin(auth.uid()));
CREATE POLICY tipos_contrato_select ON tipos_contrato FOR SELECT USING (true);
CREATE POLICY tipos_contrato_update ON tipos_contrato FOR UPDATE USING (is_rrhh_admin(auth.uid())) WITH CHECK (is_rrhh_admin(auth.uid()));

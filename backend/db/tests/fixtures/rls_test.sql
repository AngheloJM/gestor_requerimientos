-- SOLO PARA TESTS. Emula lo mínimo de Supabase para probar la PROPAGACIÓN de identidad:
-- rol `authenticated`, `auth.uid()` y políticas RLS SIMPLIFICADAS.
-- Estas políticas NO son las de producción (que viven en la base real con
-- is_rrhh_admin, puede_ver_servicio, p2_acceso, etc.) y NO deben copiarse a una migración.

DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'authenticated') THEN
        CREATE ROLE authenticated NOLOGIN;
    END IF;
END $$;

-- En la imagen supabase/postgres auth.uid() ya existe: solo se crea si falta (Postgres "pelado").
DO $$ BEGIN
    IF to_regprocedure('auth.uid()') IS NULL THEN
        CREATE SCHEMA IF NOT EXISTS auth;
        CREATE FUNCTION auth.uid() RETURNS uuid LANGUAGE sql STABLE AS $f$
            SELECT (nullif(current_setting('request.jwt.claims', true), '')::jsonb ->> 'sub')::uuid
        $f$;
        GRANT USAGE ON SCHEMA auth TO authenticated;
        GRANT EXECUTE ON FUNCTION auth.uid() TO authenticated;
    END IF;
END $$;

CREATE FUNCTION test_rol() RETURNS text LANGUAGE sql STABLE SECURITY DEFINER AS $$
    SELECT rol FROM public.usuarios WHERE id = auth.uid()
$$;
CREATE FUNCTION test_ve_servicio(sid uuid) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER AS $$
    SELECT test_rol() = 'rrhh_admin' OR EXISTS (
        SELECT 1 FROM public.usuario_servicios WHERE usuario_id = auth.uid() AND servicio_id = sid)
$$;

GRANT USAGE ON SCHEMA public TO authenticated;
GRANT SELECT, INSERT, UPDATE ON ALL TABLES IN SCHEMA public TO authenticated;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO authenticated;

ALTER TABLE usuarios                ENABLE ROW LEVEL SECURITY;
ALTER TABLE servicios               ENABLE ROW LEVEL SECURITY;
ALTER TABLE servicio_cargos         ENABLE ROW LEVEL SECURITY;
ALTER TABLE requerimientos          ENABLE ROW LEVEL SECURITY;
ALTER TABLE requerimiento_historial ENABLE ROW LEVEL SECURITY;
ALTER TABLE requerimiento_entregas  ENABLE ROW LEVEL SECURITY;
ALTER TABLE solicitudes_cambio      ENABLE ROW LEVEL SECURITY;

CREATE POLICY t_usuarios ON usuarios FOR SELECT
    USING (id = auth.uid() OR test_rol() = 'rrhh_admin');
CREATE POLICY t_servicios ON servicios FOR SELECT USING (test_ve_servicio(id));
CREATE POLICY t_sc ON servicio_cargos FOR SELECT USING (test_ve_servicio(servicio_id));
CREATE POLICY t_req_sel ON requerimientos FOR SELECT
    USING (servicio_id IS NULL AND test_rol() = 'rrhh_admin' OR test_ve_servicio(servicio_id));
CREATE POLICY t_req_upd ON requerimientos FOR UPDATE
    USING (test_rol() = 'rrhh_admin') WITH CHECK (test_rol() = 'rrhh_admin');
CREATE POLICY t_hist_sel ON requerimiento_historial FOR SELECT
    USING (EXISTS (SELECT 1 FROM requerimientos r WHERE r.id = requerimiento_id));
CREATE POLICY t_hist_ins ON requerimiento_historial FOR INSERT
    WITH CHECK (test_rol() = 'rrhh_admin' AND usuario_id = auth.uid());
CREATE POLICY t_ent ON requerimiento_entregas FOR SELECT
    USING (EXISTS (SELECT 1 FROM requerimientos r WHERE r.id = requerimiento_id));
-- Regla conocida: por defecto RR.HH. + solicitante; campo = 'centro_costo' también Costos.
CREATE POLICY t_sol ON solicitudes_cambio FOR SELECT USING (
    test_rol() = 'rrhh_admin'
    OR solicitante_id = auth.uid()
    OR (campo = 'centro_costo' AND test_rol() = 'encargado_costos'));

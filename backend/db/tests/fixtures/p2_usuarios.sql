-- SOLO PARA TESTS. Políticas RLS de producción para `usuarios` y `usuario_servicios`
-- (leídas de pg_policy el 2026-11-03). Requiere `p2_edicion` (is_rrhh_admin).
CREATE POLICY usuarios_select_authenticated ON usuarios FOR SELECT USING (true);
CREATE POLICY usuarios_insert_rrhh_admin ON usuarios FOR INSERT WITH CHECK (is_rrhh_admin(auth.uid()));
CREATE POLICY usuarios_update_rrhh_admin ON usuarios FOR UPDATE
    USING (is_rrhh_admin(auth.uid())) WITH CHECK (is_rrhh_admin(auth.uid()));
CREATE POLICY usuarios_delete_rrhh_admin ON usuarios FOR DELETE USING (is_rrhh_admin(auth.uid()));

ALTER TABLE usuario_servicios ENABLE ROW LEVEL SECURITY;
GRANT DELETE ON usuario_servicios TO authenticated;
CREATE POLICY usuario_servicios_select ON usuario_servicios FOR SELECT
    USING ((usuario_id = auth.uid()) OR is_rrhh_admin(auth.uid()));
CREATE POLICY usuario_servicios_write ON usuario_servicios FOR ALL
    USING (is_rrhh_admin(auth.uid())) WITH CHECK (is_rrhh_admin(auth.uid()));

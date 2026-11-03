-- SOLO PARA TESTS. Tabla y políticas de producción de parámetros salariales (2026-11-03),
-- con los valores reales (públicos). Requiere `p2_edicion` (is_rrhh_admin).
CREATE TABLE parametros_salariales_historial (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    parametro text NOT NULL,
    valor numeric NOT NULL,
    vigente_desde date NOT NULL,
    creado_por uuid,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT uq_parametro_vigencia UNIQUE (parametro, vigente_desde)
);
ALTER TABLE parametros_salariales_historial ENABLE ROW LEVEL SECURITY;
GRANT SELECT, INSERT ON parametros_salariales_historial TO authenticated;
CREATE POLICY parametros_select_authenticated ON parametros_salariales_historial FOR SELECT USING (true);
CREATE POLICY parametros_insert_rrhh_admin ON parametros_salariales_historial FOR INSERT
    WITH CHECK (is_rrhh_admin(auth.uid()));

INSERT INTO parametros_salariales_historial (parametro, valor, vigente_desde) VALUES
  ('recargo_nocturno_pct', 0.25, '2020-01-01'),
  ('salario_minimo_nacional', 3300, '2026-10-26'),
  ('salario_minimo_nacional', 3600, '2026-10-28');

-- 0004: correcciones de la revisión del 04/11/2026 (docs/PENDIENTES.md, "Revisión 04/11").
-- ADITIVA: solo agrega columnas NULLABLE, tablas y políticas NUEVAS. No modifica ni borra
-- columnas, datos, políticas ni funciones existentes. Idempotente (se puede re-ejecutar).
--
-- NO aplicada en producción (Supabase): requiere confirmación explícita antes de aplicarla.

-- ---------------------------------------------------------------------------------------
-- Contratos: tiempo de renovación (cuánto antes del vencimiento empezar a renegociar).
-- Opcional e independiente de la duración del contrato.
-- ---------------------------------------------------------------------------------------
ALTER TABLE public.contratos_comerciales
    ADD COLUMN IF NOT EXISTS tiempo_renovacion_valor  integer,
    ADD COLUMN IF NOT EXISTS tiempo_renovacion_unidad text;

DO $$ BEGIN
    ALTER TABLE public.contratos_comerciales ADD CONSTRAINT contratos_tiempo_renovacion_chk CHECK (
        (tiempo_renovacion_valor IS NULL AND tiempo_renovacion_unidad IS NULL)
        OR (tiempo_renovacion_valor > 0 AND tiempo_renovacion_unidad IN ('meses', 'dias'))
    );
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

-- ---------------------------------------------------------------------------------------
-- Servicios: la capacitación son tres tiempos. `tiempo_capacitacion_dias` (existente) sigue
-- siendo "Tiempo de capacitación" y es el que usa crear_requerimiento_con_snapshot.
-- TODO(negocio): ¿cuál de los tres define la fecha de inicio de capacitación del requerimiento?
-- ---------------------------------------------------------------------------------------
ALTER TABLE public.servicios
    ADD COLUMN IF NOT EXISTS tiempo_ojt_dias                   integer,
    ADD COLUMN IF NOT EXISTS tiempo_capacitacion_ajustada_dias integer;

DO $$ BEGIN
    ALTER TABLE public.servicios ADD CONSTRAINT servicios_tiempos_capacitacion_chk CHECK (
        (tiempo_ojt_dias IS NULL OR tiempo_ojt_dias >= 0)
        AND (tiempo_capacitacion_ajustada_dias IS NULL OR tiempo_capacitacion_ajustada_dias >= 0)
    );
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

-- ---------------------------------------------------------------------------------------
-- Cargo dentro de un servicio: cantidad de mujeres y de hombres.
-- A confirmar: se guardan por cargo del servicio (servicio_cargos), no por servicio.
-- ---------------------------------------------------------------------------------------
ALTER TABLE public.servicio_cargos
    ADD COLUMN IF NOT EXISTS cantidad_mujeres integer,
    ADD COLUMN IF NOT EXISTS cantidad_hombres integer;

DO $$ BEGIN
    ALTER TABLE public.servicio_cargos ADD CONSTRAINT servicio_cargos_cantidades_chk CHECK (
        (cantidad_mujeres IS NULL OR cantidad_mujeres >= 0)
        AND (cantidad_hombres IS NULL OR cantidad_hombres >= 0)
    );
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

-- ---------------------------------------------------------------------------------------
-- Composición salarial de cargos compartidos entre varios centros de costo:
-- pares (centro de costo, monto). El % de cada centro se calcula (no se guarda).
-- La regla "solo cargos con más de un centro de costo posible" se valida en la API.
-- ---------------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS public.cargo_centros_costo (
    cargo_id        uuid          NOT NULL REFERENCES public.cargos(id),
    centro_costo_id uuid          NOT NULL REFERENCES public.centros_costo(id),
    monto           numeric(12,2) NOT NULL CHECK (monto > 0),
    created_at      timestamptz   NOT NULL DEFAULT now(),
    PRIMARY KEY (cargo_id, centro_costo_id)
);

ALTER TABLE public.cargo_centros_costo ENABLE ROW LEVEL SECURITY;

-- Mismo criterio que servicio_cargos: escribe solo RR.HH.
-- TODO(negocio): ¿quién más ve la distribución? Hoy solo RR.HH.
DROP POLICY IF EXISTS cargo_centros_costo_select ON public.cargo_centros_costo;
CREATE POLICY cargo_centros_costo_select ON public.cargo_centros_costo
    FOR SELECT USING (public.is_rrhh_admin(auth.uid()));
DROP POLICY IF EXISTS cargo_centros_costo_write ON public.cargo_centros_costo;
CREATE POLICY cargo_centros_costo_write ON public.cargo_centros_costo
    FOR ALL USING (public.is_rrhh_admin(auth.uid())) WITH CHECK (public.is_rrhh_admin(auth.uid()));

GRANT SELECT, INSERT, UPDATE, DELETE ON public.cargo_centros_costo TO authenticated;

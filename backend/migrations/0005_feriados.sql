-- 0005: días feriados (revisión del 06/11/2026, puntos 11 y 12; ver docs/PENDIENTES.md).
-- ADITIVA: solo crea una tabla NUEVA con sus políticas. No modifica ni borra nada existente.
-- Idempotente (se puede re-ejecutar).
--
-- Uso: el tiempo de reclutamiento se cuenta en días hábiles (sin sábados, domingos ni los
-- feriados de esta tabla).
--
-- Alcance actual: feriados NACIONALES únicamente (decisión del 2026-11-07).
-- Diseño para crecer: la clave es `id`, no la fecha, y la unicidad por fecha es un índice con
-- nombre propio. Si se necesitan feriados por departamento, una migración futura agrega una
-- columna `departamento` NULLABLE (NULL = nacional) y un índice único nuevo por
-- (fecha, departamento), sin tocar las filas cargadas.
-- TODO(negocio): ¿hacen falta feriados por departamento? No implementado.
--
-- Producción: NO aplicada. Requiere confirmación explícita y respaldo previo.

CREATE TABLE IF NOT EXISTS public.feriados (
    id         uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    fecha      date        NOT NULL,
    nombre     text        NOT NULL CHECK (length(btrim(nombre)) BETWEEN 2 AND 120),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS feriados_nacional_fecha_key ON public.feriados (fecha);

ALTER TABLE public.feriados ENABLE ROW LEVEL SECURITY;

-- Lectura: cualquier usuario autenticado (el cálculo de días hábiles al crear un
-- requerimiento los necesita). Escritura: solo RR.HH., como los parámetros salariales.
DROP POLICY IF EXISTS feriados_select ON public.feriados;
CREATE POLICY feriados_select ON public.feriados
    FOR SELECT USING (auth.uid() IS NOT NULL);
DROP POLICY IF EXISTS feriados_write ON public.feriados;
CREATE POLICY feriados_write ON public.feriados
    FOR ALL USING (public.is_rrhh_admin(auth.uid())) WITH CHECK (public.is_rrhh_admin(auth.uid()));

GRANT SELECT, INSERT, UPDATE, DELETE ON public.feriados TO authenticated;

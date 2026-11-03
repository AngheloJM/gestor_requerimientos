-- 0003: roles y permisos configurables (fases 0 y 1 de docs/PLAN_ROLES.md).
-- ADITIVA: solo crea tablas, funciones, políticas y datos NUEVOS. No modifica `usuarios.rol`
-- (su CHECK sigue igual) ni ninguna política existente.
--
-- IMPORTANTE: en esta fase los permisos se REGISTRAN pero todavía NO se aplican: el acceso
-- real lo siguen decidiendo las políticas RLS actuales. Los 4 roles de sistema describen ese
-- comportamiento actual y no se editan desde aquí (fase 3 del plan).
--
-- Depende de `public.is_rrhh_admin(uuid)` (existe en la base). Administración de roles:
-- solo `rrhh_admin` (TODO(negocio): ¿rol "Administrador del sistema" aparte?).

-- ---------------------------------------------------------------------------------------
-- Catálogo de permisos (lo define el código; no se edita desde la aplicación)
-- ---------------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS public.permisos (
    clave          text PRIMARY KEY CHECK (clave ~ '^[a-z_]+\.[a-z_]+$'),
    modulo         text NOT NULL,
    recurso        text NOT NULL,
    recurso_nombre text NOT NULL,
    accion         text NOT NULL,
    descripcion    text NOT NULL,
    -- true: el permiso puede limitarse a "servicios asignados" o "propios".
    admite_alcance boolean NOT NULL DEFAULT false,
    orden          integer NOT NULL,
    UNIQUE (recurso, accion)
);

CREATE TABLE IF NOT EXISTS public.roles (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    clave       text NOT NULL UNIQUE CHECK (clave ~ '^[a-z][a-z0-9_]{1,49}$'),
    nombre      text NOT NULL CHECK (length(btrim(nombre)) BETWEEN 2 AND 80),
    descripcion text CHECK (descripcion IS NULL OR length(descripcion) <= 500),
    -- Roles que existen en `usuarios.rol`: reflejan el comportamiento actual; no editables.
    es_sistema  boolean NOT NULL DEFAULT false,
    activo      boolean NOT NULL DEFAULT true,
    creado_por  uuid,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX IF NOT EXISTS roles_nombre_uq ON public.roles (lower(btrim(nombre)));

CREATE TABLE IF NOT EXISTS public.rol_permisos (
    rol_id        uuid NOT NULL REFERENCES public.roles(id) ON DELETE CASCADE,
    permiso_clave text NOT NULL REFERENCES public.permisos(clave),
    alcance       text NOT NULL DEFAULT 'todos'
                  CHECK (alcance IN ('todos', 'servicios_asignados', 'propios')),
    PRIMARY KEY (rol_id, permiso_clave)
);

-- Auditoría de cambios de permisos (la escribe un trigger; nadie la edita).
CREATE TABLE IF NOT EXISTS public.rol_permisos_historial (
    id                uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    rol_id            uuid NOT NULL,
    permiso_clave     text NOT NULL,
    cambio            text NOT NULL CHECK (cambio IN ('otorgado', 'revocado', 'alcance')),
    alcance_anterior  text,
    alcance_nuevo     text,
    usuario_id        uuid,
    created_at        timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS rol_permisos_historial_rol_idx
    ON public.rol_permisos_historial (rol_id, created_at);

-- ---------------------------------------------------------------------------------------
-- Reglas: roles de sistema inmutables, auditoría y updated_at
-- ---------------------------------------------------------------------------------------
-- Las reglas se aplican a los roles de la aplicación (`authenticated`/`anon`); la migración,
-- que corre como dueño de la base, puede cargar los roles de sistema. Estas funciones NO son
-- SECURITY DEFINER para que `current_user` sea el de la sesión.
CREATE OR REPLACE FUNCTION public.roles_desde_app() RETURNS boolean
LANGUAGE sql STABLE AS $$ SELECT current_user IN ('authenticated', 'anon') $$;

CREATE OR REPLACE FUNCTION public.roles_proteger_sistema() RETURNS trigger
LANGUAGE plpgsql SET search_path TO 'public' AS $$
BEGIN
    IF NOT public.roles_desde_app() THEN
        RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
    END IF;
    IF TG_OP = 'INSERT' THEN
        IF NEW.es_sistema THEN RAISE EXCEPTION 'No se pueden crear roles de sistema.'; END IF;
        NEW.creado_por := auth.uid();
        RETURN NEW;
    END IF;
    IF OLD.es_sistema THEN
        RAISE EXCEPTION 'Los roles de sistema reflejan las reglas actuales y no se modifican.';
    END IF;
    IF TG_OP = 'DELETE' THEN RETURN OLD; END IF;
    NEW.es_sistema := false;
    NEW.clave := OLD.clave;
    NEW.creado_por := OLD.creado_por;
    NEW.created_at := OLD.created_at;
    NEW.updated_at := now();
    RETURN NEW;
END $$;

DROP TRIGGER IF EXISTS roles_proteger_sistema ON public.roles;
CREATE TRIGGER roles_proteger_sistema BEFORE INSERT OR UPDATE OR DELETE ON public.roles
    FOR EACH ROW EXECUTE FUNCTION public.roles_proteger_sistema();

CREATE OR REPLACE FUNCTION public.rol_permisos_auditar() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'public' AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        INSERT INTO public.rol_permisos_historial (rol_id, permiso_clave, cambio, alcance_nuevo, usuario_id)
        VALUES (NEW.rol_id, NEW.permiso_clave, 'otorgado', NEW.alcance, auth.uid());
        RETURN NEW;
    ELSIF TG_OP = 'UPDATE' THEN
        IF NEW.permiso_clave IS DISTINCT FROM OLD.permiso_clave OR NEW.rol_id IS DISTINCT FROM OLD.rol_id THEN
            RAISE EXCEPTION 'Para cambiar el permiso, quítelo y vuelva a otorgarlo.';
        END IF;
        IF NEW.alcance IS DISTINCT FROM OLD.alcance THEN
            INSERT INTO public.rol_permisos_historial
                (rol_id, permiso_clave, cambio, alcance_anterior, alcance_nuevo, usuario_id)
            VALUES (NEW.rol_id, NEW.permiso_clave, 'alcance', OLD.alcance, NEW.alcance, auth.uid());
        END IF;
        RETURN NEW;
    ELSE
        -- Al borrar el rol completo (CASCADE) también queda registrado.
        INSERT INTO public.rol_permisos_historial (rol_id, permiso_clave, cambio, alcance_anterior, usuario_id)
        VALUES (OLD.rol_id, OLD.permiso_clave, 'revocado', OLD.alcance, auth.uid());
        RETURN OLD;
    END IF;
END $$;

DROP TRIGGER IF EXISTS rol_permisos_auditar ON public.rol_permisos;
CREATE TRIGGER rol_permisos_auditar AFTER INSERT OR UPDATE OR DELETE ON public.rol_permisos
    FOR EACH ROW EXECUTE FUNCTION public.rol_permisos_auditar();

-- Validación: roles de sistema inmutables desde la aplicación; alcance solo donde se admite.
CREATE OR REPLACE FUNCTION public.rol_permisos_validar() RETURNS trigger
LANGUAGE plpgsql SET search_path TO 'public' AS $$
BEGIN
    IF public.roles_desde_app() AND EXISTS (
        SELECT 1 FROM public.roles r
        WHERE r.id = coalesce(NEW.rol_id, OLD.rol_id) AND r.es_sistema) THEN
        RAISE EXCEPTION 'Los roles de sistema reflejan las reglas actuales y no se modifican.';
    END IF;
    IF TG_OP = 'DELETE' THEN RETURN OLD; END IF;
    IF NEW.alcance <> 'todos' AND NOT EXISTS (
        SELECT 1 FROM public.permisos p WHERE p.clave = NEW.permiso_clave AND p.admite_alcance) THEN
        RAISE EXCEPTION 'El permiso % no admite alcance limitado.', NEW.permiso_clave;
    END IF;
    RETURN NEW;
END $$;

DROP TRIGGER IF EXISTS rol_permisos_validar ON public.rol_permisos;
CREATE TRIGGER rol_permisos_validar BEFORE INSERT OR UPDATE OR DELETE ON public.rol_permisos
    FOR EACH ROW EXECUTE FUNCTION public.rol_permisos_validar();

-- ---------------------------------------------------------------------------------------
-- RLS: leer = usuarios activos; administrar = rrhh_admin
-- ---------------------------------------------------------------------------------------
ALTER TABLE public.permisos               ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.roles                  ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.rol_permisos           ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.rol_permisos_historial ENABLE ROW LEVEL SECURITY;

GRANT SELECT ON public.permisos TO authenticated;
GRANT SELECT, INSERT, UPDATE, DELETE ON public.roles, public.rol_permisos TO authenticated;
GRANT SELECT ON public.rol_permisos_historial TO authenticated;
REVOKE ALL ON public.permisos, public.roles, public.rol_permisos, public.rol_permisos_historial FROM anon;

DROP POLICY IF EXISTS permisos_lectura ON public.permisos;
CREATE POLICY permisos_lectura ON public.permisos FOR SELECT TO authenticated USING (true);

DROP POLICY IF EXISTS roles_lectura ON public.roles;
CREATE POLICY roles_lectura ON public.roles FOR SELECT TO authenticated USING (true);
DROP POLICY IF EXISTS roles_alta ON public.roles;
CREATE POLICY roles_alta ON public.roles FOR INSERT TO authenticated
    WITH CHECK (public.is_rrhh_admin(auth.uid()));
DROP POLICY IF EXISTS roles_edicion ON public.roles;
CREATE POLICY roles_edicion ON public.roles FOR UPDATE TO authenticated
    USING (public.is_rrhh_admin(auth.uid())) WITH CHECK (public.is_rrhh_admin(auth.uid()));
DROP POLICY IF EXISTS roles_baja ON public.roles;
CREATE POLICY roles_baja ON public.roles FOR DELETE TO authenticated
    USING (public.is_rrhh_admin(auth.uid()));

DROP POLICY IF EXISTS rol_permisos_lectura ON public.rol_permisos;
CREATE POLICY rol_permisos_lectura ON public.rol_permisos FOR SELECT TO authenticated USING (true);
DROP POLICY IF EXISTS rol_permisos_escritura ON public.rol_permisos;
CREATE POLICY rol_permisos_escritura ON public.rol_permisos FOR ALL TO authenticated
    USING (public.is_rrhh_admin(auth.uid())) WITH CHECK (public.is_rrhh_admin(auth.uid()));

DROP POLICY IF EXISTS rol_permisos_historial_lectura ON public.rol_permisos_historial;
CREATE POLICY rol_permisos_historial_lectura ON public.rol_permisos_historial FOR SELECT
    TO authenticated USING (public.is_rrhh_admin(auth.uid()));

-- ---------------------------------------------------------------------------------------
-- Catálogo inicial (modulo, recurso, nombre del recurso, acción, descripción, alcance)
-- ---------------------------------------------------------------------------------------
INSERT INTO public.permisos (clave, modulo, recurso, recurso_nombre, accion, descripcion, admite_alcance, orden)
SELECT p.recurso || '.' || p.accion, p.modulo, p.recurso, p.recurso_nombre, p.accion, p.descripcion,
       p.alcance, p.orden
FROM (VALUES
  -- Requerimientos
  ('Requerimientos', 'requerimientos', 'Requerimientos', 'ver',       'Ver requerimientos', true, 100),
  ('Requerimientos', 'requerimientos', 'Requerimientos', 'crear',     'Crear requerimientos', true, 101),
  ('Requerimientos', 'requerimientos', 'Requerimientos', 'editar',    'Editar fechas, cantidad, motivo y comentarios', true, 102),
  ('Requerimientos', 'requerimientos', 'Requerimientos', 'eliminar',  'Eliminar requerimientos', true, 103),
  ('Requerimientos', 'requerimientos', 'Requerimientos', 'aprobar',   'Aprobar o rechazar (cambiar el estado)', true, 104),
  ('Requerimientos', 'requerimientos', 'Requerimientos', 'historial', 'Ver el historial de cambios', true, 105),
  ('Requerimientos', 'entregas', 'Entregas de personal', 'ver',       'Ver entregas', true, 110),
  ('Requerimientos', 'entregas', 'Entregas de personal', 'crear',     'Registrar entregas', true, 111),
  ('Requerimientos', 'entregas', 'Entregas de personal', 'editar',    'Rectificar entregas', true, 112),
  ('Requerimientos', 'entregas', 'Entregas de personal', 'eliminar',  'Eliminar entregas', true, 113),
  ('Requerimientos', 'entregas', 'Entregas de personal', 'decidir',   'Aceptar o rechazar entregas', true, 114),
  -- Matriz de servicio
  ('Matriz de servicio', 'clientes', 'Clientes', 'ver',      'Ver clientes', false, 200),
  ('Matriz de servicio', 'clientes', 'Clientes', 'crear',    'Crear clientes', false, 201),
  ('Matriz de servicio', 'clientes', 'Clientes', 'editar',   'Editar clientes', false, 202),
  ('Matriz de servicio', 'clientes', 'Clientes', 'eliminar', 'Eliminar clientes', false, 203),
  ('Matriz de servicio', 'proyectos', 'Proyectos', 'ver',      'Ver proyectos', false, 210),
  ('Matriz de servicio', 'proyectos', 'Proyectos', 'crear',    'Crear proyectos', false, 211),
  ('Matriz de servicio', 'proyectos', 'Proyectos', 'editar',   'Editar proyectos', false, 212),
  ('Matriz de servicio', 'proyectos', 'Proyectos', 'eliminar', 'Eliminar proyectos', false, 213),
  ('Matriz de servicio', 'centros_costo', 'Centros de costo', 'ver',      'Ver centros de costo', false, 220),
  ('Matriz de servicio', 'centros_costo', 'Centros de costo', 'crear',    'Crear centros de costo', false, 221),
  ('Matriz de servicio', 'centros_costo', 'Centros de costo', 'editar',   'Editar centros de costo', false, 222),
  ('Matriz de servicio', 'centros_costo', 'Centros de costo', 'eliminar', 'Eliminar centros de costo', false, 223),
  ('Matriz de servicio', 'contratos', 'Contratos comerciales', 'ver',      'Ver contratos', false, 230),
  ('Matriz de servicio', 'contratos', 'Contratos comerciales', 'crear',    'Crear contratos', false, 231),
  ('Matriz de servicio', 'contratos', 'Contratos comerciales', 'editar',   'Editar contratos', false, 232),
  ('Matriz de servicio', 'contratos', 'Contratos comerciales', 'eliminar', 'Eliminar contratos', false, 233),
  ('Matriz de servicio', 'servicios', 'Servicios', 'ver',      'Ver servicios', true, 240),
  ('Matriz de servicio', 'servicios', 'Servicios', 'crear',    'Crear servicios', false, 241),
  ('Matriz de servicio', 'servicios', 'Servicios', 'editar',   'Editar servicios', true, 242),
  ('Matriz de servicio', 'servicios', 'Servicios', 'eliminar', 'Desactivar servicios', false, 243),
  ('Matriz de servicio', 'cargos', 'Cargos', 'ver',      'Ver el catálogo de cargos', false, 250),
  ('Matriz de servicio', 'cargos', 'Cargos', 'crear',    'Crear cargos', false, 251),
  ('Matriz de servicio', 'cargos', 'Cargos', 'editar',   'Editar cargos', false, 252),
  ('Matriz de servicio', 'cargos', 'Cargos', 'eliminar', 'Eliminar cargos', false, 253),
  -- Composición salarial
  ('Composición salarial', 'sueldos', 'Sueldos por servicio y cargo', 'ver',        'Ver la estructura (cargos por servicio)', true, 300),
  ('Composición salarial', 'sueldos', 'Sueldos por servicio y cargo', 'ver_montos', 'Ver los montos (haberes y total ganado)', true, 301),
  ('Composición salarial', 'sueldos', 'Sueldos por servicio y cargo', 'crear',      'Agregar cargos con sueldo a un servicio', true, 302),
  ('Composición salarial', 'sueldos', 'Sueldos por servicio y cargo', 'editar',     'Modificar sueldos', true, 303),
  ('Composición salarial', 'sueldos', 'Sueldos por servicio y cargo', 'eliminar',   'Quitar cargos de un servicio', true, 304),
  ('Composición salarial', 'bonos', 'Métricas y bonos por cargo', 'ver',      'Ver métricas, bonos y escalas', true, 310),
  ('Composición salarial', 'bonos', 'Métricas y bonos por cargo', 'crear',    'Agregar métricas y bonos', true, 311),
  ('Composición salarial', 'bonos', 'Métricas y bonos por cargo', 'editar',   'Modificar montos, pesos y escalas', true, 312),
  ('Composición salarial', 'bonos', 'Métricas y bonos por cargo', 'eliminar', 'Quitar métricas y bonos', true, 313),
  ('Composición salarial', 'catalogo_metricas', 'Catálogo de métricas', 'ver',      'Ver el catálogo de métricas', false, 320),
  ('Composición salarial', 'catalogo_metricas', 'Catálogo de métricas', 'crear',    'Crear métricas', false, 321),
  ('Composición salarial', 'catalogo_metricas', 'Catálogo de métricas', 'editar',   'Editar métricas', false, 322),
  ('Composición salarial', 'catalogo_metricas', 'Catálogo de métricas', 'eliminar', 'Eliminar métricas', false, 323),
  ('Composición salarial', 'catalogo_bonos', 'Catálogo de bonos', 'ver',      'Ver el catálogo de bonos', false, 330),
  ('Composición salarial', 'catalogo_bonos', 'Catálogo de bonos', 'crear',    'Crear bonos', false, 331),
  ('Composición salarial', 'catalogo_bonos', 'Catálogo de bonos', 'editar',   'Editar bonos', false, 332),
  ('Composición salarial', 'catalogo_bonos', 'Catálogo de bonos', 'eliminar', 'Eliminar bonos', false, 333),
  ('Composición salarial', 'parametros', 'Parámetros salariales', 'ver',   'Ver salario mínimo, recargos, etc.', false, 340),
  ('Composición salarial', 'parametros', 'Parámetros salariales', 'crear', 'Registrar un nuevo valor vigente', false, 341),
  -- Personal
  ('Personal', 'operadores', 'Operadores', 'ver',      'Ver operadores', true, 400),
  ('Personal', 'operadores', 'Operadores', 'crear',    'Registrar operadores', true, 401),
  ('Personal', 'operadores', 'Operadores', 'editar',   'Editar operadores', true, 402),
  ('Personal', 'operadores', 'Operadores', 'eliminar', 'Eliminar operadores', true, 403),
  ('Personal', 'sueldos_excepcion', 'Sueldos de excepción', 'ver',      'Ver sueldos fuera de lo estándar', true, 410),
  ('Personal', 'sueldos_excepcion', 'Sueldos de excepción', 'crear',    'Cargar sueldos de excepción', true, 411),
  ('Personal', 'sueldos_excepcion', 'Sueldos de excepción', 'editar',   'Editar sueldos de excepción', true, 412),
  ('Personal', 'sueldos_excepcion', 'Sueldos de excepción', 'eliminar', 'Eliminar sueldos de excepción', true, 413),
  -- Solicitudes de cambio
  ('Solicitudes de cambio', 'solicitudes', 'Solicitudes de cambio', 'ver',      'Ver solicitudes', true, 500),
  ('Solicitudes de cambio', 'solicitudes', 'Solicitudes de cambio', 'crear',    'Pedir cambios', true, 501),
  ('Solicitudes de cambio', 'solicitudes', 'Solicitudes de cambio', 'resolver', 'Resolver o rechazar', true, 502),
  ('Solicitudes de cambio', 'solicitudes', 'Solicitudes de cambio', 'cancelar', 'Cancelar solicitudes', true, 503),
  -- Paneles
  ('Paneles', 'tablero', 'Tablero de pendientes', 'ver', 'Ver el tablero', false, 600),
  ('Paneles', 'almanaque', 'Almanaque', 'ver', 'Ver el almanaque de entradas y salidas', true, 610),
  -- Administración
  ('Administración', 'usuarios', 'Usuarios', 'ver',               'Ver usuarios', false, 700),
  ('Administración', 'usuarios', 'Usuarios', 'crear',             'Crear usuarios', false, 701),
  ('Administración', 'usuarios', 'Usuarios', 'editar',            'Editar datos de usuarios', false, 702),
  ('Administración', 'usuarios', 'Usuarios', 'eliminar',          'Eliminar usuarios', false, 703),
  ('Administración', 'usuarios', 'Usuarios', 'asignar_servicios', 'Asignar servicios a usuarios', false, 704),
  ('Administración', 'usuarios', 'Usuarios', 'asignar_rol',       'Cambiar el rol de un usuario', false, 705),
  ('Administración', 'roles', 'Roles y permisos', 'ver',      'Ver roles y permisos', false, 710),
  ('Administración', 'roles', 'Roles y permisos', 'crear',    'Crear roles', false, 711),
  ('Administración', 'roles', 'Roles y permisos', 'editar',   'Editar roles y sus permisos', false, 712),
  ('Administración', 'roles', 'Roles y permisos', 'eliminar', 'Eliminar roles', false, 713)
) AS p(modulo, recurso, recurso_nombre, accion, descripcion, alcance, orden)
ON CONFLICT (clave) DO NOTHING;

-- ---------------------------------------------------------------------------------------
-- Roles de sistema = los 4 valores actuales de `usuarios.rol`, con los permisos que hoy
-- aplican las políticas RLS (verificado en producción el 2026-11-03; ver PLAN_ROLES.md).
-- ---------------------------------------------------------------------------------------
INSERT INTO public.roles (clave, nombre, descripcion, es_sistema)
SELECT v.clave, v.nombre, v.descripcion, true FROM (VALUES
  ('rrhh_admin',       'RR.HH. · Administrador', 'Acceso total. Único rol que aprueba y edita la configuración.'),
  ('encargado_costos', 'Costos',                 'Consulta de sus servicios, operadores y solicitudes de centro de costo.'),
  ('estandar',         'Encargado de servicio',  'Pide y sigue requerimientos de sus servicios asignados.'),
  ('externo',          'Externo',                'Acceso mínimo: solo lo que pidió.')
) AS v(clave, nombre, descripcion)
WHERE NOT EXISTS (SELECT 1 FROM public.roles r WHERE r.clave = v.clave);
INSERT INTO public.rol_permisos (rol_id, permiso_clave, alcance)
    SELECT r.id, x.permiso, x.alcance
    FROM (VALUES
      -- rrhh_admin: todo
      ('rrhh_admin', NULL, 'todos'),
      -- encargado_costos
      ('encargado_costos', 'requerimientos.ver', 'servicios_asignados'),
      ('encargado_costos', 'requerimientos.crear', 'propios'),
      ('encargado_costos', 'requerimientos.editar', 'propios'),
      ('encargado_costos', 'requerimientos.historial', 'servicios_asignados'),
      ('encargado_costos', 'entregas.ver', 'propios'),
      ('encargado_costos', 'entregas.crear', 'propios'),
      ('encargado_costos', 'clientes.ver', 'todos'),
      ('encargado_costos', 'proyectos.ver', 'todos'),
      ('encargado_costos', 'centros_costo.ver', 'todos'),
      ('encargado_costos', 'contratos.ver', 'todos'),
      ('encargado_costos', 'servicios.ver', 'servicios_asignados'),
      ('encargado_costos', 'cargos.ver', 'todos'),
      ('encargado_costos', 'sueldos.ver', 'servicios_asignados'),
      ('encargado_costos', 'sueldos.ver_montos', 'servicios_asignados'),
      ('encargado_costos', 'bonos.ver', 'servicios_asignados'),
      ('encargado_costos', 'catalogo_metricas.ver', 'todos'),
      ('encargado_costos', 'catalogo_bonos.ver', 'todos'),
      ('encargado_costos', 'parametros.ver', 'todos'),
      ('encargado_costos', 'operadores.ver', 'todos'),
      ('encargado_costos', 'sueldos_excepcion.ver', 'todos'),
      ('encargado_costos', 'solicitudes.ver', 'propios'),
      ('encargado_costos', 'solicitudes.crear', 'servicios_asignados'),
      ('encargado_costos', 'solicitudes.resolver', 'todos'),  -- solo campo = centro_costo
      ('encargado_costos', 'solicitudes.cancelar', 'propios'),
      ('encargado_costos', 'tablero.ver', 'todos'),
      ('encargado_costos', 'almanaque.ver', 'servicios_asignados'),
      -- estandar
      ('estandar', 'requerimientos.ver', 'servicios_asignados'),
      ('estandar', 'requerimientos.crear', 'servicios_asignados'),
      ('estandar', 'requerimientos.editar', 'servicios_asignados'),
      ('estandar', 'requerimientos.historial', 'servicios_asignados'),
      ('estandar', 'entregas.ver', 'servicios_asignados'),
      ('estandar', 'entregas.crear', 'servicios_asignados'),
      ('estandar', 'clientes.ver', 'todos'),
      ('estandar', 'proyectos.ver', 'todos'),
      ('estandar', 'centros_costo.ver', 'todos'),
      ('estandar', 'contratos.ver', 'todos'),
      ('estandar', 'servicios.ver', 'servicios_asignados'),
      ('estandar', 'cargos.ver', 'todos'),
      ('estandar', 'sueldos.ver', 'servicios_asignados'),
      -- TODO(negocio): ¿estandar ve montos? Hoy RLS le muestra las filas completas.
      ('estandar', 'sueldos.ver_montos', 'servicios_asignados'),
      ('estandar', 'bonos.ver', 'servicios_asignados'),
      ('estandar', 'catalogo_metricas.ver', 'todos'),
      ('estandar', 'parametros.ver', 'todos'),
      ('estandar', 'solicitudes.ver', 'propios'),
      ('estandar', 'solicitudes.crear', 'servicios_asignados'),
      ('estandar', 'solicitudes.cancelar', 'propios'),
      ('estandar', 'tablero.ver', 'todos'),
      ('estandar', 'almanaque.ver', 'servicios_asignados'),
      -- externo
      ('externo', 'requerimientos.ver', 'propios'),
      ('externo', 'cargos.ver', 'todos'),
      ('externo', 'catalogo_metricas.ver', 'todos'),
      ('externo', 'parametros.ver', 'todos')
    ) AS v(rol, permiso, alcance)
    JOIN public.roles r ON r.clave = v.rol
    CROSS JOIN LATERAL (
        SELECT p.clave AS permiso,
               CASE WHEN p.admite_alcance THEN v.alcance ELSE 'todos' END AS alcance
        FROM public.permisos p WHERE v.permiso IS NULL OR p.clave = v.permiso
    ) x
ON CONFLICT DO NOTHING;

-- 0002: autenticación propia del backend (reemplaza el login de Supabase Auth).
-- ADITIVA: solo crea tablas nuevas; no modifica ni borra nada existente.
--
--   credenciales: email + hash argon2id por usuario (`usuarios.id`).
--   sesiones:     refresh tokens (solo su hash SHA-256), con rotación y revocación.
--
-- Estas tablas NO deben exponerse por la API de Supabase (PostgREST): sin privilegios para
-- anon/authenticated y RLS activo sin políticas. Solo las usa el backend con su propio rol.

CREATE TABLE IF NOT EXISTS public.credenciales (
    usuario_id     uuid PRIMARY KEY REFERENCES public.usuarios(id) ON DELETE CASCADE,
    email          text NOT NULL CHECK (email = lower(email) AND position('@' IN email) > 1),
    password_hash  text NOT NULL,
    creado_en      timestamptz NOT NULL DEFAULT now(),
    actualizado_en timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX IF NOT EXISTS credenciales_email_uq ON public.credenciales (email);

CREATE TABLE IF NOT EXISTS public.sesiones (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    usuario_id      uuid NOT NULL REFERENCES public.usuarios(id) ON DELETE CASCADE,
    token_hash      bytea NOT NULL UNIQUE,
    creada_en       timestamptz NOT NULL DEFAULT now(),
    expira_en       timestamptz NOT NULL,
    revocada_en     timestamptz,
    reemplazada_por uuid REFERENCES public.sesiones(id),
    user_agent      text,
    ip              inet
);
CREATE INDEX IF NOT EXISTS sesiones_usuario_idx ON public.sesiones (usuario_id) WHERE revocada_en IS NULL;

ALTER TABLE public.credenciales ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.sesiones     ENABLE ROW LEVEL SECURITY;
REVOKE ALL ON public.credenciales, public.sesiones FROM PUBLIC;
DO $$
DECLARE r text;
BEGIN
    FOREACH r IN ARRAY ARRAY['anon', 'authenticated', 'service_role'] LOOP
        IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = r) THEN
            EXECUTE format('REVOKE ALL ON public.credenciales, public.sesiones FROM %I', r);
        END IF;
    END LOOP;
END $$;

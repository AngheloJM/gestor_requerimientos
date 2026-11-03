-- SOLO PARA TESTS. Subconjunto del schema real de Supabase (nombres de tablas/columnas
-- reales; los TIPOS son aproximados porque no se exportó el dump). No es una migración.

-- gen_random_uuid() es nativo desde Postgres 13 (no requiere pgcrypto).

CREATE TABLE clientes (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    nombre text NOT NULL UNIQUE,
    logo_url text,
    manual_marca_url text,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE centros_costo (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    codigo text NOT NULL UNIQUE,
    nombre text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE contratos_comerciales (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    codigo text NOT NULL UNIQUE,
    nombre text NOT NULL,
    cliente_id uuid REFERENCES clientes(id),
    fecha_inicio date NOT NULL,
    fecha_fin date,
    duracion_valor integer,
    duracion_unidad text,
    CHECK (fecha_fin IS NULL OR (duracion_valor IS NULL AND duracion_unidad IS NULL))
);
CREATE TABLE proyectos (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    cliente_id uuid NOT NULL REFERENCES clientes(id),
    nombre text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE servicios (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    codigo text UNIQUE,
    nombre text NOT NULL,
    centro_costo_id uuid REFERENCES centros_costo(id),
    contrato_comercial_id uuid REFERENCES contratos_comerciales(id),
    cliente_id uuid REFERENCES clientes(id),
    proyecto_id uuid REFERENCES proyectos(id),
    tipo_contrato text, modalidad text, tipo_turno text, dias_trabajo text[],
    departamento_region text, manual_funciones_codigo text, tiempo_capacitacion_dias integer,
    capacitacion_pagada boolean, tiene_refrigerio boolean,
    activo boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE cargos (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    nombre text NOT NULL,
    tipo text,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE servicio_cargos (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    servicio_id uuid REFERENCES servicios(id),
    cargo_id uuid NOT NULL REFERENCES cargos(id),
    haber_basico_diurno numeric(12,2) NOT NULL,
    haber_basico_nocturno numeric(12,2),
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE catalogo_metricas (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    nombre text NOT NULL UNIQUE,
    concepto text,
    formula text,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE servicio_cargo_metricas (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    servicio_cargo_id uuid NOT NULL REFERENCES servicio_cargos(id),
    metrica_id uuid NOT NULL REFERENCES catalogo_metricas(id),
    peso_pct numeric NOT NULL,
    monto_bono numeric NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE servicio_cargo_metrica_umbrales (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    servicio_cargo_metrica_id uuid NOT NULL REFERENCES servicio_cargo_metricas(id),
    intervalo_min numeric,
    intervalo_max numeric,
    pct_bono numeric NOT NULL,
    observaciones text,
    orden integer NOT NULL
);
CREATE TABLE usuarios (
    id uuid PRIMARY KEY,
    nombre text,
    cargo_id uuid REFERENCES cargos(id),
    division text,
    region text,
    inmediato_superior_id uuid REFERENCES usuarios(id),
    rol text NOT NULL,
    activo boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now(),
    es_reclutador_staff_default boolean NOT NULL DEFAULT false
);
CREATE TABLE usuario_servicios (
    usuario_id uuid NOT NULL REFERENCES usuarios(id),
    servicio_id uuid NOT NULL REFERENCES servicios(id),
    PRIMARY KEY (usuario_id, servicio_id)
);
CREATE TABLE requerimientos (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    numero bigint GENERATED ALWAYS AS IDENTITY,
    folio text,
    solicitante_id uuid,
    servicio_id uuid REFERENCES servicios(id),
    cargo_id uuid NOT NULL REFERENCES cargos(id),
    tipo text,
    cantidad_personas integer,
    motivo text,
    fecha_inicio date NOT NULL,
    fecha_fin_tentativa date,
    haber_basico_diurno_snap numeric, haber_basico_nocturno_snap numeric, bonos_snap jsonb,
    tiempo_capacitacion_snap integer, comentarios_adicionales text,
    reclutador_asignado_id uuid,
    estado text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    fecha_inicio_capacitacion date,
    fecha_inicio_operacion date,
    fecha_fin date,
    fecha_salida date
);
CREATE TABLE requerimiento_historial (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    requerimiento_id uuid NOT NULL REFERENCES requerimientos(id),
    campo text NOT NULL,
    valor_anterior text,
    valor_nuevo text,
    usuario_id uuid,
    created_at timestamptz NOT NULL DEFAULT now(),
    glosa text,
    grupo_edicion uuid,
    categoria_desvio text,
    detalle_desvio text
);
CREATE TABLE requerimiento_entregas (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    requerimiento_id uuid NOT NULL REFERENCES requerimientos(id),
    cargo_id uuid NOT NULL REFERENCES cargos(id),
    fecha_entrega date NOT NULL,
    cantidad integer NOT NULL,
    cantidad_reclutada integer,
    tipo text NOT NULL,
    corrige_entrega_id uuid REFERENCES requerimiento_entregas(id),
    glosa text NOT NULL,
    categoria_desvio text,
    detalle_desvio text,
    registrado_por uuid NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE requerimiento_entrega_decisiones (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    entrega_id uuid NOT NULL UNIQUE REFERENCES requerimiento_entregas(id),
    estado text NOT NULL,
    comentario text,
    decidido_por uuid NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE solicitudes_cambio (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    servicio_id uuid NOT NULL REFERENCES servicios(id),
    cargo_id uuid REFERENCES cargos(id),
    campo text NOT NULL,
    solicitante_id uuid NOT NULL,
    motivo text NOT NULL,
    descripcion text,
    estado text NOT NULL,
    responsable_id uuid,
    comentario_resolucion text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    cerrada_at timestamptz
);

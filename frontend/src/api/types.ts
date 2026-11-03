/**
 * Tipos de respuesta de la API (espejo de los DTOs de `backend/api/src/*.rs`).
 * Mantener sincronizados a mano hasta que se genere el contrato (ej. OpenAPI).
 *
 * La autorización la decide RLS en Postgres: un usuario sin permiso recibe listas vacías
 * (no ve filas) o 403/404 en operaciones puntuales.
 */

export type Uuid = string
/** Fecha ISO "YYYY-MM-DD". */
export type FechaIso = string
/** Timestamp RFC 3339 en UTC. */
export type Timestamp = string
/** Decimal como string (ej. "3750.50"). NO convertir a number para calcular. */
export type Monto = string

export type Rol = 'rrhh_admin' | 'estandar' | 'encargado_costos' | 'externo'

export interface Ref {
  id: Uuid
  /** Puede venir null si RLS no permite ver la entidad referenciada. */
  nombre: string | null
}

// GET /me
export interface Me {
  user_id: Uuid
  nombre: string | null
  rol: Rol | string | null
  activo: boolean | null
}

// GET /matriz-servicio — orden: Centro de Costo → Cliente → Servicio → Cargo
export interface FiltrosMatriz {
  servicio_id?: Uuid | null
  cliente_id?: Uuid | null
  centro_costo_id?: Uuid | null
  proyecto_id?: Uuid | null
}

export interface Umbral {
  intervalo_min: string | null
  intervalo_max: string | null
  pct_bono: string
  observaciones: string | null
  orden: number
}

export interface Metrica {
  id: Uuid
  metrica_id: Uuid
  nombre: string
  concepto: string | null
  formula: string | null
  peso_pct: string
  monto_bono: Monto
  umbrales: Umbral[]
}

export interface FilaMatriz {
  centro_costo: { id: Uuid; codigo: string; nombre: string } | null
  cliente: Ref | null
  proyecto: Ref | null
  servicio: { id: Uuid; codigo: string | null; nombre: string }
  cargo: { id: Uuid; nombre: string; tipo: string | null }
  servicio_cargo_id: Uuid
  haber_basico_diurno: Monto
  haber_basico_nocturno: Monto | null
  total_haberes: Monto
  /** null hasta que exista el resultado mensual de métricas (pendiente de negocio). */
  total_ganado: Monto | null
  metricas: Metrica[]
}

// GET /almanaque?mes=&anio=
export interface LineaAlmanaque {
  servicio: Ref | null
  cargo: Ref
  cantidad: number
  /** Registros agrupados con cantidad NULL (no sumados). */
  registros_sin_cantidad: number
}

export interface DiaAlmanaque {
  fecha: FechaIso
  entradas: LineaAlmanaque[]
  salidas: LineaAlmanaque[]
  entregas: LineaAlmanaque[]
  rectificaciones: LineaAlmanaque[]
}

export interface Almanaque {
  anio: number
  mes: number
  dias: DiaAlmanaque[]
}

// GET /aprobaciones · PATCH /aprobaciones/{id} · GET /aprobaciones/{id}/historial
export interface Aprobacion {
  id: Uuid
  numero: number
  folio: string | null
  tipo: string | null
  estado: string | null
  servicio: Ref | null
  cargo: Ref
  cantidad: number | null
  fecha_inicio: FechaIso
  fecha_fin_tentativa: FechaIso | null
  fecha_fin: FechaIso | null
  ultima_modificacion: { usuario_id: Uuid | null; fecha: Timestamp } | null
}

export interface EditarFechaFin {
  id: Uuid
  /** null = quitar la fecha de fin. */
  fecha_fin: FechaIso | null
  glosa?: string
}

export interface EntradaHistorial {
  id: string
  campo: string
  valor_anterior: string | null
  valor_nuevo: string | null
  usuario_id: Uuid | null
  created_at: Timestamp
  glosa: string | null
  grupo_edicion: string | null
  categoria_desvio: string | null
  detalle_desvio: string | null
}

// GET /tablero
export type Area = 'comercial' | 'operaciones' | 'rrhh'

export interface ServicioIncompleto {
  servicio: Ref
  cliente: string | null
  tareas: { codigo: string; area: Area }[]
}

export interface TarjetaKanban {
  id: Uuid
  servicio: Ref
  cargo: Ref | null
  campo: string
  solicitante_id: Uuid | null
  motivo: string | null
  descripcion: string | null
  estado: string | null
  responsable_id: Uuid | null
  comentario_resolucion: string | null
  created_at: Timestamp
  updated_at: Timestamp | null
  cerrada_at: Timestamp | null
}

export interface Tablero {
  indicadores: {
    servicios_total: number
    servicios_configurados: number
    servicios_incompletos: number
    areas_sin_completar: number
  }
  /** Criterios de completitud aún no evaluados (pendientes de definición). */
  criterios_sin_definir: string[]
  servicios_incompletos: ServicioIncompleto[]
  /** Kanban desde solicitudes_cambio. */
  kanban: Record<Area, TarjetaKanban[]>
}

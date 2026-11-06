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
  cargo: string | null
  division: string | null
  region: string | null
  inmediato_superior: string | null
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
  /** A confirmar: cantidades por cargo del servicio. */
  cantidad_mujeres: number | null
  cantidad_hombres: number | null
  /** Tiempos de capacitación del cargo en este servicio (días). */
  tiempo_capacitacion_dias: number | null
  tiempo_ojt_dias: number | null
  tiempo_capacitacion_ajustada_dias: number | null
  total_haberes: Monto
  /** Haberes + suma de monto_bono configurados (sin ajustar por cumplimiento real). */
  total_ganado: Monto
  metricas: Metrica[]
}

// GET /almanaque?mes=&anio=  (fuente: leer_eventos_almanaque + solo servicios activos)
export type EstadoEntrega = 'pendiente' | 'aceptada' | 'rechazada'

export interface LineaAlmanaque {
  cliente: string | null
  /** null si el requerimiento no tiene servicio. */
  servicio: string | null
  cargo: string | null
  /** Solo entregas y rectificaciones. */
  estado_entrega: EstadoEntrega | string | null
  /** Suma; las rectificaciones pueden ser negativas. */
  cantidad: number
  /** Registros agrupados con cantidad NULL (no sumados). */
  registros_sin_cantidad: number
  folios: string[]
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

// GET /requerimientos[?mios=true]
/** Valores de la base (trigger p2_validar_campos); filas antiguas podrían traer otros. */
export type EstadoRequerimiento = 'pendiente' | 'en_proceso' | 'aprobado' | 'rechazado' | 'cerrado'

export interface Requerimiento {
  id: Uuid
  numero: number
  folio: string | null
  /** El usuario actual es el solicitante. */
  es_mio: boolean
  servicio: Ref | null
  cargo: Ref
  tipo: string | null
  motivo: string | null
  cantidad: number | null
  estado: EstadoRequerimiento | string | null
  fecha_inicio: FechaIso
  fecha_inicio_operacion: FechaIso | null
  creado: Timestamp
}

// GET /requerimientos/{id} — detalle de solo lectura
/** Bono congelado al crear: configurado `{metrica, monto}` o extra `{tipo:'extra', nombre, monto, motivo}`. */
export interface BonoCongelado {
  metrica?: string
  tipo?: string
  nombre?: string
  monto?: number | string
  motivo?: string
}

export interface RequerimientoDetalle {
  id: Uuid
  numero: number
  folio: string | null
  tipo: string | null
  estado: EstadoRequerimiento | string | null
  motivo: string | null
  cantidad: number | null
  fecha_inicio: FechaIso
  fecha_fin_tentativa: FechaIso | null
  fecha_inicio_capacitacion: FechaIso | null
  fecha_inicio_operacion: FechaIso | null
  comentarios: string | null
  /** Congelados al crear. */
  haber_basico_diurno: Monto | null
  haber_basico_nocturno: Monto | null
  bonos: BonoCongelado[] | null
  tiempo_capacitacion_dias: number | null
  creado: Timestamp
  /** updated_at con microsegundos: devolverlo tal cual al editar. */
  version: Timestamp
  /** Con cuánto arrancó el requerimiento (no se edita). */
  cantidad_original: number | null
  /** Suma de entregas aceptadas. */
  total_aceptado: number
  /** Suma de todas las entregas registradas (a confirmar). */
  reclutados_informados: number
  solicitante_nombre: string | null
  solicitante_cargo: string | null
  solicitante_division: string | null
  solicitante_region: string | null
  inmediato_superior: string | null
  /** Datos actuales del servicio. */
  servicio_nombre: string | null
  servicio_codigo: string | null
  cargo_nombre: string | null
  cargo_tipo: string | null
  centro_costo: string | null
  cliente: string | null
  contrato: string | null
  tipo_contrato: string | null
  modalidad: string | null
  tipo_turno: string | null
  dias_trabajo: string[] | null
  departamento_region: string | null
}

// GET /requerimientos/opciones · POST /requerimientos
export type TipoRequerimiento = 'nuevo' | 'modificacion' | 'promocion' | 'cambio'
/** `cargos.tipo` en la base. */
export type TipoCargo = 'operativo' | 'administrativo_staff'

export interface CargoOpcion {
  id: Uuid
  nombre: string
  tipo: TipoCargo | string | null
  /** Configurado en el servicio (referencia para requerimientos de crecimiento). */
  cantidad_mujeres: number | null
  cantidad_hombres: number | null
}

export interface ServicioOpcion {
  id: Uuid
  nombre: string
  tiempo_capacitacion_dias: number | null
  cargos: CargoOpcion[]
}

export interface OpcionesRequerimiento {
  servicios: ServicioOpcion[]
  /** Cargos administrativo_staff: se pueden pedir sin servicio. */
  cargos_staff: CargoOpcion[]
  tipos: TipoRequerimiento[]
  motivos: string[]
}

export interface BonoExtra {
  nombre: string
  /** Decimal como string, hasta 2 decimales. */
  monto: Monto
  motivo: string
}

export interface CrearRequerimiento {
  tipo: TipoRequerimiento
  servicio_id: Uuid | null
  cargo_id: Uuid
  cantidad: number
  motivo: string
  fecha_inicio_capacitacion: FechaIso
  fecha_inicio_operacion: FechaIso
  fecha_fin: FechaIso | null
  comentarios: string | null
  bonos_extra: BonoExtra[]
}

// GET /permisos · /roles (fases 0 y 1: los permisos se registran, todavía no se aplican)
export type Alcance = 'todos' | 'servicios_asignados' | 'propios'

export interface Permiso {
  clave: string
  modulo: string
  recurso: string
  recurso_nombre: string
  accion: string
  descripcion: string
  admite_alcance: boolean
}

export interface PermisoAsignado {
  clave: string
  alcance: Alcance
}

export interface RolConfigurable {
  id: Uuid
  clave: string
  nombre: string
  descripcion: string | null
  /** Los 4 valores de usuarios.rol: reflejan las reglas actuales, no se editan. */
  es_sistema: boolean
  activo: boolean
  permisos: PermisoAsignado[]
  actualizado: Timestamp
}

export interface GuardarRol {
  nombre: string
  descripcion: string | null
  activo: boolean
  permisos: PermisoAsignado[]
}

// GET /usuarios · GET /usuarios/opciones · PUT /usuarios/{id}  (editar: solo RR.HH., RLS)
export interface UsuarioAdmin {
  id: Uuid
  nombre: string
  cargo: Ref | null
  division: string | null
  region: string | null
  inmediato_superior: Ref | null
  rol: Rol | string
  activo: boolean
  servicios: Ref[]
}

export interface OpcionesUsuario {
  cargos: Ref[]
  /** Servicios activos. */
  servicios: Ref[]
  /** Un usuario = un rol (CHECK de la base). */
  roles: Rol[]
}

export interface GuardarUsuario {
  cargo_id: Uuid | null
  division: string | null
  region: string | null
  inmediato_superior_id: Uuid | null
  rol: Rol
  activo: boolean
  servicios: Uuid[]
}

// GET/POST /parametros-salariales (cada cambio agrega una fila; alta solo RR.HH.)
export interface ValorParametro {
  id: Uuid
  parametro: string
  /** Decimal como string (ej. "3600", "0.25"). */
  valor: string
  vigente_desde: FechaIso
  registrado: Timestamp
}

export interface Parametros {
  /** Fecha (zona del negocio) usada para calcular los vigentes. */
  fecha: FechaIso
  vigentes: ValorParametro[]
  historial: ValorParametro[]
}

export interface NuevoValorParametro {
  parametro: string
  valor: string
  vigente_desde: FechaIso
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
  fecha_inicio_operacion: FechaIso | null
  fecha_inicio_capacitacion: FechaIso | null
  motivo: string | null
  /** Nombre del solicitante (null si no es visible). */
  solicitante: string | null
  ultima_modificacion: { usuario_id: Uuid | null; fecha: Timestamp } | null
  /** updated_at con microsegundos: devolverlo tal cual al editar (control de concurrencia). */
  version: Timestamp
}

/** Categorías de desvío admitidas por la base (p2_validar_desvio). */
export type CategoriaDesvio =
  | 'falla_capacitacion'
  | 'rechazo_operaciones'
  | 'ratio_reclutamiento'
  | 'cambio_cliente'

/** Al menos `fecha_fin` o `estado`. La glosa queda en el historial. */
export interface EditarRequerimiento {
  id: Uuid
  /** Ausente = no tocar; null = quitar la fecha de fin. */
  fecha_fin?: FechaIso | null
  /** Solo RR.HH. (lo valida la base). */
  estado?: EstadoRequerimiento
  version: Timestamp
  /** Obligatoria: 3 a 2000 caracteres. */
  glosa: string
  /** Van juntas: categoría + explicación (3 a 2000 caracteres). */
  categoria_desvio?: CategoriaDesvio
  detalle_desvio?: string
}

/** PATCH /requerimientos/{id}: cambia la cantidad actual (glosa obligatoria). */
export interface EditarCantidad {
  id: Uuid
  cantidad: number
  version: Timestamp
  glosa: string
  categoria_desvio?: CategoriaDesvio
  detalle_desvio?: string
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

export interface ServicioConfigurado {
  servicio: Ref
  cliente: string | null
}

export interface ServicioIncompleto {
  servicio: Ref
  cliente: string | null
  /** % de criterios de completitud cumplidos (0–100). */
  porcentaje: number
  /** Contrato asignado (si es visible), para ir a corregirlo. */
  contrato_id: Uuid | null
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
  servicios_configurados: ServicioConfigurado[]
  servicios_incompletos: ServicioIncompleto[]
  /** Kanban desde solicitudes_cambio. */
  kanban: Record<Area, TarjetaKanban[]>
}

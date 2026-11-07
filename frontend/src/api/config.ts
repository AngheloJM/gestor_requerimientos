/**
 * Configuración de la Matriz de Servicio (`/config/*`). Escritura solo RR.HH. (RLS); las
 * reglas las valida la base (triggers) y la API (formatos).
 */
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'

import { api } from '@/api/instance'
import type { FechaIso, Uuid } from '@/api/types'

export type RecursoConfig =
  | 'clientes'
  | 'proyectos'
  | 'centros-costo'
  | 'servicios'
  | 'cargos'
  | 'contratos'
  | 'catalogo-metricas'
  | 'catalogo-bonos'

export interface ClienteCfg {
  id: Uuid
  nombre: string
  /** TODO(negocio): hoy URL/texto; subir el archivo no está implementado. */
  logo_url: string | null
  manual_marca_url: string | null
}

export interface ProyectoCfg {
  id: Uuid
  cliente_id: Uuid
  cliente_nombre: string | null
  nombre: string
}

export interface CentroCostoCfg {
  id: Uuid
  codigo: string
  nombre: string
}

export interface ServicioCfg {
  id: Uuid
  codigo: string | null
  nombre: string
  centro_costo_id: Uuid | null
  centro_costo_nombre: string | null
  contrato_comercial_id: Uuid | null
  contrato_nombre: string | null
  cliente_id: Uuid | null
  cliente_nombre: string | null
  proyecto_id: Uuid | null
  proyecto_nombre: string | null
  tipo_contrato: string | null
  modalidad: string
  tipo_turno: string | null
  dias_trabajo: string[]
  departamento_region: string | null
  manual_funciones_codigo: string | null
  tiempo_capacitacion_dias: number | null
  capacitacion_pagada: boolean | null
  tiene_refrigerio: boolean | null
  activo: boolean
}

export interface CargoCfg {
  id: Uuid
  nombre: string
  tipo: 'operativo' | 'administrativo_staff' | string
}

export interface ContratoCfg {
  id: Uuid
  codigo: string
  nombre: string | null
  cliente_id: Uuid | null
  cliente_nombre: string | null
  fecha_inicio: FechaIso | null
  fecha_fin: FechaIso | null
  duracion_valor: number | null
  duracion_unidad: 'meses' | 'dias' | string | null
  /** Opcional: cuánto antes del vencimiento empezar a renegociar. */
  tiempo_renovacion_valor: number | null
  tiempo_renovacion_unidad: 'meses' | 'dias' | string | null
}

export interface MetricaCatalogoCfg {
  id: Uuid
  nombre: string
  concepto: string | null
  formula: string | null
}

export interface BonoCatalogoCfg {
  id: Uuid
  nombre: string
}

export interface CatalogosServicio {
  tipos_contrato: string[]
  modalidades: string[]
  tipos_turno: string[]
  dias: string[]
}

export interface UmbralCfg {
  intervalo_min: string | null
  intervalo_max: string | null
  /** TODO(negocio): escala 0–100 o 0–1 sin confirmar; se guarda tal cual. */
  pct_bono: string
  observaciones: string | null
  orden: number
}

export interface MetricaAsignada {
  id: Uuid
  metrica_id: Uuid
  nombre: string
  peso_pct: string
  monto_bono: string
  umbrales: UmbralCfg[]
}

const clave = (recurso: string) => ['config', recurso] as const

export function useConfigLista<T>(recurso: RecursoConfig) {
  return useQuery({
    queryKey: clave(recurso),
    queryFn: ({ signal }) => api.get<T[]>(`/config/${recurso}`, undefined, signal),
  })
}

export function useCatalogosServicio() {
  return useQuery({
    queryKey: ['config', 'catalogos-servicio'] as const,
    queryFn: ({ signal }) => api.get<CatalogosServicio>('/config/catalogos-servicio', undefined, signal),
    staleTime: 10 * 60_000,
  })
}

/** Invalida la configuración y lo que depende de ella (matriz, opciones de formularios). */
function invalidarTodo(qc: ReturnType<typeof useQueryClient>) {
  void qc.invalidateQueries({ queryKey: ['config'] })
  void qc.invalidateQueries({ queryKey: ['matriz-servicio'] })
  void qc.invalidateQueries({ queryKey: ['requerimientos', 'opciones'] })
  void qc.invalidateQueries({ queryKey: ['tablero'] })
}

/** Crea (sin id) o actualiza (con id) un registro de la sección. */
export function useConfigGuardar(recurso: RecursoConfig) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, body }: { id?: string; body: unknown }) =>
      id
        ? api.put<void>(`/config/${recurso}/${encodeURIComponent(id)}`, body).then(() => ({ id }))
        : api.post<{ id: Uuid }>(`/config/${recurso}`, body),
    onSuccess: () => invalidarTodo(qc),
  })
}

export function useConfigEliminar(recurso: RecursoConfig) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api.del<void>(`/config/${recurso}/${encodeURIComponent(id)}`),
    onSuccess: () => invalidarTodo(qc),
  })
}

// --- Composición salarial y métricas por cargo ---------------------------------------------

export function useGuardarComposicion() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, body }: { id?: string; body: Record<string, unknown> }) =>
      id
        ? api.put<void>(`/config/composiciones/${encodeURIComponent(id)}`, body).then(() => ({ id }))
        : api.post<{ id: Uuid }>('/config/composiciones', body),
    onSuccess: () => invalidarTodo(qc),
  })
}

// --- Composición compartida (cargo en varios centros de costo) ------------------------------

export interface MontoCentroCfg {
  centro_costo_id: Uuid
  centro_costo_nombre: string
  monto: string
  /** % sobre el total, 2 decimales. */
  porcentaje: string
}

export interface ComposicionCompartida {
  cargo_id: Uuid
  cargo_nombre?: string
  /** Centros de los servicios activos donde está el cargo. */
  centros_posibles: { id: Uuid; nombre: string }[]
  /** false: el cargo tiene un solo centro de costo posible (o ninguno): no se puede guardar. */
  compartido: boolean
  /** El cargo no está en ningún servicio activo: se reparte entre cualquier centro de costo. */
  sin_servicio: boolean
  distribucion: MontoCentroCfg[]
  total: string
}

export function useCompartidas() {
  return useQuery({
    queryKey: ['config', 'composicion-compartida'] as const,
    queryFn: ({ signal }) => api.get<ComposicionCompartida[]>('/config/composicion-compartida', undefined, signal),
  })
}

export function useCompartida(cargoId: string) {
  return useQuery({
    queryKey: ['config', 'composicion-compartida', cargoId] as const,
    queryFn: ({ signal }) =>
      api.get<ComposicionCompartida>(`/config/composicion-compartida/${encodeURIComponent(cargoId)}`, undefined, signal),
    enabled: !!cargoId,
  })
}

export function useGuardarCompartida() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ cargoId, centros }: { cargoId: string; centros: { centro_costo_id: Uuid; monto: string }[] }) =>
      api.put<ComposicionCompartida>(`/config/composicion-compartida/${encodeURIComponent(cargoId)}`, { centros }),
    onSuccess: () => invalidarTodo(qc),
  })
}

export function useMetricasAsignadas(servicioCargoId: string | null) {
  return useQuery({
    queryKey: ['config', 'metricas-asignadas', servicioCargoId] as const,
    queryFn: ({ signal }) =>
      api.get<MetricaAsignada[]>(`/config/composiciones/${encodeURIComponent(servicioCargoId!)}/metricas`, undefined, signal),
    enabled: !!servicioCargoId,
  })
}

export function useAsignarMetrica(servicioCargoId: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: { metrica_id: Uuid; peso_pct: string; monto_bono: string }) =>
      api.post<{ id: Uuid }>(`/config/composiciones/${encodeURIComponent(servicioCargoId)}/metricas`, body),
    onSuccess: () => invalidarTodo(qc),
  })
}

export function useEditarMetricaAsignada() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, body }: { id: string; body: { peso_pct: string; monto_bono: string; umbrales: UmbralCfg[] } }) =>
      api.put<void>(`/config/metricas-asignadas/${encodeURIComponent(id)}`, body),
    onSuccess: () => invalidarTodo(qc),
  })
}

export function useQuitarMetricaAsignada() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api.del<void>(`/config/metricas-asignadas/${encodeURIComponent(id)}`),
    onSuccess: () => invalidarTodo(qc),
  })
}

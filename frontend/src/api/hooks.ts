/**
 * Hooks de TanStack Query para cada endpoint. Reemplazan las llamadas a
 * `supabase.from(...)` del código generado por Lovable.
 */
import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'

import { api } from '@/api/instance'
import type {
  Almanaque,
  Aprobacion,
  EditarFechaFin,
  EntradaHistorial,
  FilaMatriz,
  FiltrosMatriz,
  Me,
  Requerimiento,
  Tablero,
} from '@/api/types'

/** Claves de cache centralizadas (para invalidar de forma consistente). */
export const queryKeys = {
  me: ['me'] as const,
  matriz: (f: FiltrosMatriz) => ['matriz-servicio', f] as const,
  almanaque: (anio: number, mes: number) => ['almanaque', anio, mes] as const,
  almanaqueTodos: ['almanaque'] as const,
  requerimientos: (mios: boolean) => ['requerimientos', { mios }] as const,
  requerimientosTodos: ['requerimientos'] as const,
  aprobaciones: ['aprobaciones'] as const,
  historial: (id: string) => ['aprobaciones', id, 'historial'] as const,
  tablero: ['tablero'] as const,
}

export function useMe() {
  return useQuery({
    queryKey: queryKeys.me,
    queryFn: ({ signal }) => api.get<Me>('/me', undefined, signal),
    staleTime: 5 * 60_000,
  })
}

export function useMatrizServicio(filtros: FiltrosMatriz = {}) {
  return useQuery({
    queryKey: queryKeys.matriz(filtros),
    queryFn: ({ signal }) => api.get<FilaMatriz[]>('/matriz-servicio', { ...filtros }, signal),
    placeholderData: keepPreviousData, // evita parpadeo al cambiar filtros
  })
}

export function useAlmanaque(anio: number, mes: number) {
  return useQuery({
    queryKey: queryKeys.almanaque(anio, mes),
    queryFn: ({ signal }) => api.get<Almanaque>('/almanaque', { anio, mes }, signal),
    placeholderData: keepPreviousData,
  })
}

/** Requerimientos visibles (RLS). `mios` = solo los que pidió el usuario. */
export function useRequerimientos(mios: boolean) {
  return useQuery({
    queryKey: queryKeys.requerimientos(mios),
    queryFn: ({ signal }) =>
      api.get<Requerimiento[]>('/requerimientos', mios ? { mios: 'true' } : undefined, signal),
    placeholderData: keepPreviousData,
  })
}

export function useTablero() {
  return useQuery({
    queryKey: queryKeys.tablero,
    queryFn: ({ signal }) => api.get<Tablero>('/tablero', undefined, signal),
  })
}

/** Qué requerimientos ve cada usuario lo decide RLS. */
export function useAprobaciones(opts: { enabled?: boolean } = {}) {
  return useQuery({
    queryKey: queryKeys.aprobaciones,
    queryFn: ({ signal }) => api.get<Aprobacion[]>('/aprobaciones', undefined, signal),
    enabled: opts.enabled ?? true,
  })
}

/** Historial completo (requerimiento_historial) de un requerimiento. */
export function useHistorialAprobacion(id: string | null) {
  return useQuery({
    queryKey: queryKeys.historial(id ?? ''),
    queryFn: ({ signal }) =>
      api.get<EntradaHistorial[]>(`/aprobaciones/${encodeURIComponent(id!)}/historial`, undefined, signal),
    enabled: id !== null,
  })
}

/**
 * Edita fecha_fin (queda en requerimiento_historial). Al terminar actualiza la fila en cache e
 * invalida su historial y el almanaque.
 */
export function useEditarFechaFin() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, ...body }: EditarFechaFin) =>
      api.patch<Aprobacion>(`/aprobaciones/${encodeURIComponent(id)}`, body),
    onSuccess: (actualizada) => {
      qc.setQueryData<Aprobacion[]>(queryKeys.aprobaciones, (prev) =>
        prev?.map((a) => (a.id === actualizada.id ? actualizada : a)),
      )
      void qc.invalidateQueries({ queryKey: queryKeys.historial(actualizada.id) })
      void qc.invalidateQueries({ queryKey: queryKeys.almanaqueTodos })
      void qc.invalidateQueries({ queryKey: queryKeys.requerimientosTodos })
    },
  })
}

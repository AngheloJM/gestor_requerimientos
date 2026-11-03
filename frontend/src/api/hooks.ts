/**
 * Hooks de TanStack Query para cada endpoint. Reemplazan las llamadas a
 * `supabase.from(...)` del código generado por Lovable.
 */
import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'

import { api } from '@/api/instance'
import type {
  Almanaque,
  Aprobacion,
  EditarRequerimiento,
  EntradaHistorial,
  FilaMatriz,
  FiltrosMatriz,
  GuardarRol,
  GuardarUsuario,
  CrearRequerimiento,
  Me,
  NuevoValorParametro,
  OpcionesRequerimiento,
  OpcionesUsuario,
  Parametros,
  Permiso,
  Requerimiento,
  RolConfigurable,
  UsuarioAdmin,
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
  permisos: ['permisos'] as const,
  roles: ['roles'] as const,
  usuarios: ['usuarios'] as const,
  parametros: ['parametros-salariales'] as const,
  rol: (id: string) => ['roles', id] as const,
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

export function useOpcionesRequerimiento() {
  return useQuery({
    queryKey: ['requerimientos', 'opciones'] as const,
    queryFn: ({ signal }) =>
      api.get<OpcionesRequerimiento>('/requerimientos/opciones', undefined, signal),
    staleTime: 60_000,
  })
}

/** Crea el requerimiento (la base valida todo) y devuelve su folio. */
export function useCrearRequerimiento() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: CrearRequerimiento) => api.post<{ folio: string }>('/requerimientos', body),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: queryKeys.requerimientosTodos })
      void qc.invalidateQueries({ queryKey: queryKeys.almanaqueTodos })
    },
  })
}

/** Resolver / rechazar / cancelar una solicitud de cambio (reglas en la base). */
export function useCerrarSolicitud() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, accion, comentario }: { id: string; accion: 'resolver' | 'rechazar' | 'cancelar'; comentario?: string }) =>
      api.post<void>(`/solicitudes-cambio/${encodeURIComponent(id)}/cierre`, { accion, comentario: comentario || null }),
    onSuccess: () => void qc.invalidateQueries({ queryKey: queryKeys.tablero }),
  })
}

export function useCrearSolicitud() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: { servicio_id: string; cargo_id: string | null; campo: string; motivo: string; descripcion: string | null }) =>
      api.post<{ id: string }>('/solicitudes-cambio', body),
    onSuccess: () => void qc.invalidateQueries({ queryKey: queryKeys.tablero }),
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
 * Edita estado (aprobar/rechazar) y/o fecha_fin vía la función de la base; queda en
 * requerimiento_historial. Actualiza la fila en cache e invalida historial, almanaque y listas.
 */
export function useEditarRequerimiento() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, ...body }: EditarRequerimiento) =>
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

// --- Roles y permisos --------------------------------------------------------------------

export function usePermisos() {
  return useQuery({
    queryKey: queryKeys.permisos,
    queryFn: ({ signal }) => api.get<Permiso[]>('/permisos', undefined, signal),
    staleTime: 10 * 60_000,
  })
}

export function useRoles() {
  return useQuery({
    queryKey: queryKeys.roles,
    queryFn: ({ signal }) => api.get<RolConfigurable[]>('/roles', undefined, signal),
  })
}

export function useRol(id: string | undefined) {
  return useQuery({
    queryKey: queryKeys.rol(id ?? ''),
    queryFn: ({ signal }) => api.get<RolConfigurable>(`/roles/${encodeURIComponent(id!)}`, undefined, signal),
    enabled: !!id,
  })
}

/** Crea (sin id) o actualiza (con id) un rol con sus permisos. */
export function useGuardarRol(id?: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: GuardarRol) =>
      id
        ? api.put<RolConfigurable>(`/roles/${encodeURIComponent(id)}`, body)
        : api.post<RolConfigurable>('/roles', body),
    onSuccess: (rol) => {
      qc.setQueryData(queryKeys.rol(rol.id), rol)
      void qc.invalidateQueries({ queryKey: queryKeys.roles, exact: true })
    },
  })
}

export function useEliminarRol() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api.del<void>(`/roles/${encodeURIComponent(id)}`),
    onSuccess: (_, id) => {
      qc.removeQueries({ queryKey: queryKeys.rol(id) })
      void qc.invalidateQueries({ queryKey: queryKeys.roles, exact: true })
    },
  })
}

// --- Usuarios (administración) -----------------------------------------------------------

export function useUsuarios() {
  return useQuery({
    queryKey: queryKeys.usuarios,
    queryFn: ({ signal }) => api.get<UsuarioAdmin[]>('/usuarios', undefined, signal),
  })
}

export function useOpcionesUsuario() {
  return useQuery({
    queryKey: ['usuarios', 'opciones'] as const,
    queryFn: ({ signal }) => api.get<OpcionesUsuario>('/usuarios/opciones', undefined, signal),
    staleTime: 60_000,
  })
}

export function useGuardarUsuario(id: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: GuardarUsuario) => api.put<UsuarioAdmin>(`/usuarios/${encodeURIComponent(id)}`, body),
    onSuccess: (u) => {
      qc.setQueryData<UsuarioAdmin[]>(queryKeys.usuarios, (prev) => prev?.map((x) => (x.id === u.id ? u : x)))
      void qc.invalidateQueries({ queryKey: queryKeys.me })
    },
  })
}

// --- Parámetros salariales ---------------------------------------------------------------

export function useParametros() {
  return useQuery({
    queryKey: queryKeys.parametros,
    queryFn: ({ signal }) => api.get<Parametros>('/parametros-salariales', undefined, signal),
    staleTime: 5 * 60_000,
  })
}

export function useRegistrarParametro() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: NuevoValorParametro) => api.post<void>('/parametros-salariales', body),
    onSuccess: () => void qc.invalidateQueries({ queryKey: queryKeys.parametros }),
  })
}

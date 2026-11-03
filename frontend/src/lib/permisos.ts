import type { Alcance, Permiso, PermisoAsignado } from '@/api/types'

export const ACCIONES: Record<string, string> = {
  ver: 'Ver',
  crear: 'Crear',
  editar: 'Editar',
  eliminar: 'Eliminar',
  aprobar: 'Aprobar',
  historial: 'Ver historial',
  decidir: 'Decidir',
  ver_montos: 'Ver montos',
  resolver: 'Resolver',
  cancelar: 'Cancelar',
  asignar_servicios: 'Asignar servicios',
  asignar_rol: 'Asignar rol',
}

/** Columnas fijas de la matriz; el resto de acciones se muestran como "otras". */
export const BASICAS = ['ver', 'crear', 'editar', 'eliminar'] as const

export const ALCANCES: Record<Alcance, string> = {
  todos: 'Todos',
  servicios_asignados: 'Sus servicios',
  propios: 'Solo propios',
}

export interface Recurso {
  recurso: string
  nombre: string
  permisos: Permiso[]
  admiteAlcance: boolean
}

export interface Modulo {
  modulo: string
  recursos: Recurso[]
}

/** Agrupa el catálogo (ya ordenado) en módulos → recursos, respetando el orden. */
export function agrupar(catalogo: Permiso[]): Modulo[] {
  const modulos: Modulo[] = []
  for (const p of catalogo) {
    let m = modulos.find((x) => x.modulo === p.modulo)
    if (!m) modulos.push((m = { modulo: p.modulo, recursos: [] }))
    let r = m.recursos.find((x) => x.recurso === p.recurso)
    if (!r) m.recursos.push((r = { recurso: p.recurso, nombre: p.recurso_nombre, permisos: [], admiteAlcance: false }))
    r.permisos.push(p)
    r.admiteAlcance ||= p.admite_alcance
  }
  return modulos
}

/** Selección: clave de permiso → alcance. */
export type Seleccion = Map<string, Alcance>

export function desdeAsignados(asignados: PermisoAsignado[]): Seleccion {
  return new Map(asignados.map((a) => [a.clave, a.alcance]))
}

export function aAsignados(sel: Seleccion, catalogo: Permiso[]): PermisoAsignado[] {
  // En el orden del catálogo, y sin alcance limitado donde no se admite.
  return catalogo
    .filter((p) => sel.has(p.clave))
    .map((p) => ({ clave: p.clave, alcance: p.admite_alcance ? sel.get(p.clave)! : 'todos' }))
}

/** Alcance actual del recurso: el de su primer permiso marcado (o "todos"). */
export function alcanceDe(sel: Seleccion, r: Recurso): Alcance {
  for (const p of r.permisos) {
    const a = sel.get(p.clave)
    if (a && p.admite_alcance) return a
  }
  return 'todos'
}

export function alternar(sel: Seleccion, p: Permiso, r: Recurso): Seleccion {
  const s = new Map(sel)
  if (s.has(p.clave)) s.delete(p.clave)
  else s.set(p.clave, p.admite_alcance ? alcanceDe(sel, r) : 'todos')
  return s
}

export function fijarAlcance(sel: Seleccion, r: Recurso, alcance: Alcance): Seleccion {
  const s = new Map(sel)
  for (const p of r.permisos) if (s.has(p.clave) && p.admite_alcance) s.set(p.clave, alcance)
  return s
}

/** "todo": marca todo el módulo; "ver": solo lecturas (ver, ver_montos, historial); "nada": limpia. */
export function aplicarAModulo(sel: Seleccion, m: Modulo, modo: 'todo' | 'ver' | 'nada'): Seleccion {
  const s = new Map(sel)
  for (const r of m.recursos) {
    const alcance = alcanceDe(sel, r)
    for (const p of r.permisos) {
      const lectura = p.accion === 'ver' || p.accion === 'ver_montos' || p.accion === 'historial'
      if (modo === 'nada' || (modo === 'ver' && !lectura)) s.delete(p.clave)
      else s.set(p.clave, p.admite_alcance ? alcance : 'todos')
    }
  }
  return s
}

export function contar(sel: Seleccion, m: Modulo): { marcados: number; total: number } {
  const permisos = m.recursos.flatMap((r) => r.permisos)
  return { marcados: permisos.filter((p) => sel.has(p.clave)).length, total: permisos.length }
}

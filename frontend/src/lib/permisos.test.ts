import { describe, expect, it } from 'vitest'

import type { Permiso } from '@/api/types'
import { aAsignados, agrupar, alternar, aplicarAModulo, contar, desdeAsignados, fijarAlcance } from '@/lib/permisos'

const p = (modulo: string, recurso: string, accion: string, alcance = true): Permiso => ({
  clave: `${recurso}.${accion}`,
  modulo,
  recurso,
  recurso_nombre: recurso,
  accion,
  descripcion: '',
  admite_alcance: alcance,
})

const catalogo = [
  p('Req', 'requerimientos', 'ver'),
  p('Req', 'requerimientos', 'crear'),
  p('Req', 'requerimientos', 'aprobar'),
  p('Req', 'requerimientos', 'historial'),
  p('Matriz', 'clientes', 'ver', false),
  p('Matriz', 'clientes', 'eliminar', false),
]

describe('permisos', () => {
  const [req, matriz] = agrupar(catalogo)

  it('agrupa por módulo y recurso respetando el orden', () => {
    expect([req.modulo, matriz.modulo]).toEqual(['Req', 'Matriz'])
    expect(req.recursos[0].permisos.map((x) => x.accion)).toEqual(['ver', 'crear', 'aprobar', 'historial'])
    expect(matriz.recursos[0].admiteAlcance).toBe(false)
  })

  it('alternar hereda el alcance del recurso y fijarAlcance lo cambia en bloque', () => {
    const r = req.recursos[0]
    let s = alternar(new Map(), catalogo[0], r)
    s = fijarAlcance(s, r, 'propios')
    s = alternar(s, catalogo[2], r)
    expect(aAsignados(s, catalogo)).toEqual([
      { clave: 'requerimientos.ver', alcance: 'propios' },
      { clave: 'requerimientos.aprobar', alcance: 'propios' },
    ])
    expect(alternar(s, catalogo[0], r).has('requerimientos.ver')).toBe(false)
  })

  it('aplica "solo ver", "todo" y "nada" a un módulo', () => {
    let s = aplicarAModulo(new Map(), req, 'ver')
    expect([...s.keys()]).toEqual(['requerimientos.ver', 'requerimientos.historial'])
    s = aplicarAModulo(s, req, 'todo')
    expect(contar(s, req)).toEqual({ marcados: 4, total: 4 })
    s = aplicarAModulo(s, req, 'nada')
    expect(contar(s, req).marcados).toBe(0)
  })

  it('no envía alcance limitado en permisos que no lo admiten', () => {
    const s = desdeAsignados([{ clave: 'clientes.ver', alcance: 'propios' }])
    expect(aAsignados(s, catalogo)).toEqual([{ clave: 'clientes.ver', alcance: 'todos' }])
  })
})

import { describe, expect, it } from 'vitest'

import type { Me, TarjetaKanban } from '@/api/types'
import { accionesPermitidas } from '@/lib/solicitudes'

const tarjeta = (campo: string, solicitante: string, estado = 'pendiente') =>
  ({ campo, solicitante_id: solicitante, estado }) as TarjetaKanban
const me = (rol: string, id = 'yo') => ({ user_id: id, rol }) as Me

describe('accionesPermitidas', () => {
  it('sigue las reglas de la base', () => {
    expect(accionesPermitidas(tarjeta('bonos', 'otro'), me('rrhh_admin'))).toEqual({ cerrar: true, cancelar: false })
    expect(accionesPermitidas(tarjeta('centro_costo', 'otro'), me('encargado_costos'))).toEqual({ cerrar: true, cancelar: false })
    expect(accionesPermitidas(tarjeta('bonos', 'otro'), me('encargado_costos'))).toEqual({ cerrar: false, cancelar: false })
    expect(accionesPermitidas(tarjeta('bonos', 'yo'), me('estandar'))).toEqual({ cerrar: false, cancelar: true })
    expect(accionesPermitidas(tarjeta('bonos', 'yo', 'resuelta'), me('rrhh_admin', 'yo'))).toEqual({ cerrar: false, cancelar: false })
  })
})

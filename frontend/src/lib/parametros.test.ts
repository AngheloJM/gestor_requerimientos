import { describe, expect, it } from 'vitest'

import { formatParametro, nombreParametro } from '@/lib/parametros'

describe('parámetros', () => {
  it('nombra y formatea los parámetros conocidos', () => {
    expect(nombreParametro('salario_minimo_nacional')).toBe('Salario Mínimo Nacional')
    expect(formatParametro('salario_minimo_nacional', '3600')).toBe('Bs 3.600,00')
    expect(formatParametro('recargo_nocturno_pct', '0.25')).toBe('25 %')
    expect(nombreParametro('otro')).toBe('otro')
    expect(formatParametro('otro', '7')).toBe('7')
  })
})

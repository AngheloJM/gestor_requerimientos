import { describe, expect, it } from 'vitest'

import { formatFecha, formatMonto, restarDias } from '@/lib/format'

describe('format', () => {
  it('formatMonto mantiene los decimales exactos', () => {
    expect(formatMonto('3750.50')).toMatch(/^3.?750,50$/)
    expect(formatMonto('0.00')).toBe('0,00')
  })
  it('formatFecha no depende de la zona horaria', () => {
    expect(formatFecha('2026-11-01')).toBe('01/11/2026')
    expect(formatFecha(null)).toBe('—')
  })
})

describe('restarDias', () => {
  it('resta días cruzando meses y años', () => {
    expect(restarDias('2026-12-15', 10)).toBe('2026-12-05')
    expect(restarDias('2026-03-05', 10)).toBe('2026-02-23')
    expect(restarDias('2027-01-03', 5)).toBe('2026-12-29')
  })
})

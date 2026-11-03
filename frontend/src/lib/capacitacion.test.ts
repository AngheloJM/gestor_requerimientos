import { describe, expect, it } from 'vitest'

import { capacitacionCalculada } from '@/lib/capacitacion'

describe('capacitacionCalculada', () => {
  it('aplica solo a cargos operativos que no son promoción, con días > 0', () => {
    expect(capacitacionCalculada('operativo', 'nuevo', 10, '2026-12-15')).toBe('2026-12-05')
    expect(capacitacionCalculada('operativo', 'promocion', 10, '2026-12-15')).toBeNull()
    expect(capacitacionCalculada('administrativo_staff', 'nuevo', 10, '2026-12-15')).toBeNull()
    expect(capacitacionCalculada('operativo', 'nuevo', 0, '2026-12-15')).toBeNull()
    expect(capacitacionCalculada('operativo', 'nuevo', null, '2026-12-15')).toBeNull()
    expect(capacitacionCalculada('operativo', 'nuevo', 10, '')).toBeNull()
  })
})

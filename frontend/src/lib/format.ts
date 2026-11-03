import type { FechaIso, Monto } from '@/api/types'

const montoFmt = new Intl.NumberFormat('es-BO', {
  minimumFractionDigits: 2,
  maximumFractionDigits: 2,
})

/**
 * Formatea un monto decimal (string) para mostrar. Intl acepta strings numéricos y los
 * formatea sin pasar por float, así "0.1" + formato no introduce errores.
 */
export function formatMonto(m: Monto): string {
  return montoFmt.format(m as Intl.StringNumericLiteral)
}

/** "2026-11-03" → "03/11/2026" sin pasar por Date (evita corrimientos por zona horaria). */
export function formatFecha(f: FechaIso | null | undefined): string {
  if (!f) return '—'
  const [y, m, d] = f.split('-')
  return `${d}/${m}/${y}`
}

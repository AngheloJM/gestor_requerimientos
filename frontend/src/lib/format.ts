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

/** "2026-12-15" − 10 días → "2026-12-05", en calendario puro (UTC, sin zona horaria). */
export function restarDias(f: FechaIso, dias: number): FechaIso {
  const [y, m, d] = f.split('-').map(Number)
  return new Date(Date.UTC(y, m - 1, d - dias)).toISOString().slice(0, 10)
}

const MESES = [
  'enero', 'febrero', 'marzo', 'abril', 'mayo', 'junio',
  'julio', 'agosto', 'septiembre', 'octubre', 'noviembre', 'diciembre',
]

/** (2026, 11) → "Noviembre 2026". */
export function nombreMes(anio: number, mes: number): string {
  const m = MESES[mes - 1] ?? String(mes)
  return `${m.charAt(0).toUpperCase()}${m.slice(1)} ${anio}`
}

/** Día de la semana con lunes = 0, en calendario puro (UTC). */
export function diaSemanaLunes(f: FechaIso): number {
  const [y, m, d] = f.split('-').map(Number)
  return (new Date(Date.UTC(y, m - 1, d)).getUTCDay() + 6) % 7
}

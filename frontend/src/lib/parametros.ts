/** Claves conocidas de `parametros_salariales_historial` (verificadas en producción). */
export const SALARIO_MINIMO_NACIONAL = 'salario_minimo_nacional'

const ETIQUETAS: Record<string, { nombre: string; formato: 'bs' | 'pct' }> = {
  salario_minimo_nacional: { nombre: 'Salario Mínimo Nacional', formato: 'bs' },
  recargo_nocturno_pct: { nombre: 'Recargo nocturno', formato: 'pct' },
}

export const nombreParametro = (p: string) => ETIQUETAS[p]?.nombre ?? p

/** "3600" → "Bs 3.600,00"; "0.25" (pct) → "25 %"; otros, tal cual. */
export function formatParametro(p: string, valor: string): string {
  const f = ETIQUETAS[p]?.formato
  if (f === 'bs') return `Bs ${new Intl.NumberFormat('es-BO', { minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(valor as Intl.StringNumericLiteral)}`
  if (f === 'pct') return `${new Intl.NumberFormat('es-BO', { maximumFractionDigits: 2 }).format(Number(valor) * 100)} %`
  return valor
}

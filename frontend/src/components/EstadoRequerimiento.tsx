import { EstadoBadge, type Tono } from '@/components/EstadoBadge'

/** Estados de `requerimientos.estado` (CHECK de la base) → etiqueta y color. */
export const ESTADOS_REQUERIMIENTO: Record<string, { etiqueta: string; tono: Tono }> = {
  pendiente: { etiqueta: 'Pendiente', tono: 'advertencia' },
  en_proceso: { etiqueta: 'En proceso', tono: 'info' },
  aprobado: { etiqueta: 'Aprobado', tono: 'exito' },
  rechazado: { etiqueta: 'Rechazado', tono: 'error' },
  cerrado: { etiqueta: 'Cerrado', tono: 'neutral' },
}

/** Badge del estado; un valor desconocido se muestra tal cual. */
export function EstadoRequerimiento({ valor }: { valor: string | null }) {
  if (!valor) return <EstadoBadge>Sin estado</EstadoBadge>
  const e = ESTADOS_REQUERIMIENTO[valor]
  return <EstadoBadge tono={e?.tono ?? 'neutral'}>{e?.etiqueta ?? valor}</EstadoBadge>
}

/** `requerimientos.tipo` (CHECK de la base) → etiqueta. */
const TIPOS: Record<string, string> = {
  nuevo: 'Nuevo',
  modificacion: 'Modificación',
  promocion: 'Promoción',
  cambio: 'Cambio',
}

export const etiquetaTipo = (t: string | null | undefined) => (t ? (TIPOS[t] ?? t) : '—')

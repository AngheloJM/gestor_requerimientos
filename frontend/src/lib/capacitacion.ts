import type { FechaIso, TipoRequerimiento } from '@/api/types'
import { restarDias } from '@/lib/format'

/**
 * Espejo, SOLO para mostrar, de la regla de la base (`crear_requerimiento_con_snapshot`):
 * cargo operativo + tipo distinto de promoción + servicio con días de capacitación > 0
 * ⇒ inicio de capacitación = inicio de operación − días. La base es quien la aplica.
 */
export function capacitacionCalculada(
  tipoCargo: string | null | undefined,
  tipo: TipoRequerimiento | undefined,
  dias: number | null | undefined,
  inicioOperacion: FechaIso | undefined,
): FechaIso | null {
  if (tipoCargo !== 'operativo' || tipo === 'promocion' || !dias || dias <= 0) return null
  if (!inicioOperacion || !/^\d{4}-\d{2}-\d{2}$/.test(inicioOperacion)) return null
  return restarDias(inicioOperacion, dias)
}

import type { Me, TarjetaKanban } from '@/api/types'

/** `solicitudes_cambio.campo` (CHECK de la base) → etiqueta. */
export const CAMPOS_SOLICITUD: Record<string, string> = {
  centro_costo: 'Centro de costo',
  cliente: 'Cliente',
  contrato_comercial: 'Contrato comercial',
  tipo_contrato: 'Tipo de contrato',
  modalidad: 'Modalidad',
  dias_trabajo: 'Días de trabajo',
  sueldo_basico: 'Sueldo básico',
  bonos: 'Bonos',
}

/** Campos que exigen indicar el cargo (CHECK `solicitudes_cargo_requerido`). */
export const CAMPOS_CON_CARGO = ['sueldo_basico', 'bonos']

export const ESTADOS_SOLICITUD: Record<string, string> = {
  pendiente: 'Pendiente',
  resuelta: 'Resuelta',
  rechazada: 'Rechazada',
  cancelada: 'Cancelada',
}

/**
 * Qué botones mostrar. Espejo, SOLO para la interfaz, de `validar_solicitud_cambio`:
 * resolver/rechazar = RR.HH., o Costos si el campo es centro de costo; cancelar = quien la pidió.
 * La base vuelve a validarlo.
 */
export function accionesPermitidas(t: TarjetaKanban, me: Me | undefined) {
  if (t.estado !== 'pendiente' || !me) return { cerrar: false, cancelar: false }
  const responsable = me.rol === 'rrhh_admin' || (me.rol === 'encargado_costos' && t.campo === 'centro_costo')
  return { cerrar: responsable, cancelar: t.solicitante_id === me.user_id }
}

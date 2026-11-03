/** Etiquetas de los roles de `usuarios.rol` (un usuario = un rol). */
export const ETIQUETAS_ROL: Record<string, string> = {
  rrhh_admin: 'RR.HH. · Administrador',
  encargado_costos: 'Costos',
  estandar: 'Encargado de servicio',
  externo: 'Externo',
}

export const etiquetaRol = (rol: string | null | undefined) => (rol ? (ETIQUETAS_ROL[rol] ?? rol) : '')

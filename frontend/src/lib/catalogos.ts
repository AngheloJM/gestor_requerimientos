/** Departamentos de Bolivia (valores usados en `servicios.departamento_region`). */
export const DEPARTAMENTOS = [
  'Beni',
  'Chuquisaca',
  'Cochabamba',
  'La Paz',
  'Oruro',
  'Pando',
  'Potosí',
  'Santa Cruz',
  'Tarija',
] as const

/** Opciones [valor, texto] a partir de una lista de textos. */
export const pares = (xs: readonly string[] | undefined): [string, string][] => (xs ?? []).map((x) => [x, x])

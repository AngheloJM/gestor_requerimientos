/**
 * Variables de entorno públicas (Vite expone solo las VITE_*). Todo esto termina en el bundle
 * del navegador: NUNCA poner secretos aquí.
 */
export const env = {
  /** Base de la API. Por defecto `/api` (proxy de Vite en desarrollo, mismo origen). */
  apiUrl: (import.meta.env.VITE_API_URL || '/api').replace(/\/+$/, ''),
} as const

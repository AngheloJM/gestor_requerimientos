import { createApiClient } from '@/api/client'
import { getAccessToken, refrescar } from '@/auth/sesion'
import { env } from '@/lib/env'

/** Instancia única del cliente de la API. */
export const api = createApiClient({
  baseUrl: env.apiUrl,
  getToken: async () => getAccessToken(),
  onUnauthorized: refrescar,
})

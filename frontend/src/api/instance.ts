import { createApiClient } from '@/api/client'
import { env } from '@/lib/env'
import { getAccessToken } from '@/lib/supabase'

/** Instancia única del cliente de la API. */
export const api = createApiClient({ baseUrl: env.apiUrl, getToken: getAccessToken })

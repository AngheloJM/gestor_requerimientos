import { createClient } from '@supabase/supabase-js'

import { env } from '@/lib/env'

/**
 * Cliente de Supabase SOLO para autenticación (login, logout, sesión/refresh del token).
 * Los datos se obtienen exclusivamente de la API en Rust (`@/api`): no usar
 * `supabase.from(...)` ni `supabase.rpc(...)` en la aplicación.
 */
export const supabase = createClient(env.supabaseUrl, env.supabaseAnonKey, {
  auth: { persistSession: true, autoRefreshToken: true },
})

/** Access token vigente (supabase-js lo refresca automáticamente) o null si no hay sesión. */
export async function getAccessToken(): Promise<string | null> {
  const { data } = await supabase.auth.getSession()
  return data.session?.access_token ?? null
}

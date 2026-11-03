/**
 * Variables de entorno públicas (Vite expone solo las que empiezan con VITE_).
 * Todo lo que está aquí termina en el bundle del navegador: NUNCA poner secretos
 * (service_role key, JWT secret, DATABASE_URL). La anon/publishable key de Supabase es
 * pública por diseño.
 */
function requerida(nombre: string, valor: string | undefined): string {
  if (!valor) {
    throw new Error(`Falta la variable de entorno ${nombre} (ver frontend/.env.example)`)
  }
  return valor
}

export const env = {
  apiUrl: requerida('VITE_API_URL', import.meta.env.VITE_API_URL).replace(/\/+$/, ''),
  supabaseUrl: requerida('VITE_SUPABASE_URL', import.meta.env.VITE_SUPABASE_URL),
  supabaseAnonKey: requerida('VITE_SUPABASE_ANON_KEY', import.meta.env.VITE_SUPABASE_ANON_KEY),
} as const

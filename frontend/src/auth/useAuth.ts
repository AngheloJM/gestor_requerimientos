import type { Session } from '@supabase/supabase-js'
import { createContext, use } from 'react'

export interface AuthState {
  session: Session | null
  /** true mientras se recupera la sesión persistida al cargar la app. */
  cargando: boolean
  signIn: (email: string, password: string) => Promise<void>
  signOut: () => Promise<void>
}

export const AuthContext = createContext<AuthState | null>(null)

export function useAuth(): AuthState {
  const ctx = use(AuthContext)
  if (!ctx) throw new Error('useAuth debe usarse dentro de <AuthProvider>')
  return ctx
}

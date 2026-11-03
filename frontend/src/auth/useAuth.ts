import { createContext, use } from 'react'

export interface AuthState {
  autenticado: boolean
  /** true mientras se intenta recuperar la sesión (cookie de refresh) al cargar la app. */
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

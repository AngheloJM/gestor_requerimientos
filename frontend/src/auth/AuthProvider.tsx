import { useQueryClient } from '@tanstack/react-query'
import { type ReactNode, useEffect, useState } from 'react'

import { alCambiarSesion, login, logout, refrescar } from '@/auth/sesion'
import { type AuthState, AuthContext } from '@/auth/useAuth'

export function AuthProvider({ children }: { children: ReactNode }) {
  const qc = useQueryClient()
  const [autenticado, setAutenticado] = useState(false)
  const [cargando, setCargando] = useState(true)

  useEffect(() => {
    const off = alCambiarSesion((a) => {
      setAutenticado(a)
      // Al cambiar de usuario o cerrar sesión no deben quedar datos en cache.
      qc.clear()
    })
    // Recupera la sesión si hay cookie de refresh válida.
    void refrescar().finally(() => setCargando(false))
    return () => {
      off()
    }
  }, [qc])

  const value: AuthState = {
    autenticado,
    cargando,
    signIn: login,
    signOut: logout,
  }

  return <AuthContext value={value}>{children}</AuthContext>
}

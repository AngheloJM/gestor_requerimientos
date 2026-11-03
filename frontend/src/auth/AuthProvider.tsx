import type { Session } from '@supabase/supabase-js'
import { useQueryClient } from '@tanstack/react-query'
import { type ReactNode, useEffect, useState } from 'react'

import { type AuthState, AuthContext } from '@/auth/useAuth'
import { supabase } from '@/lib/supabase'

export function AuthProvider({ children }: { children: ReactNode }) {
  const qc = useQueryClient()
  const [session, setSession] = useState<Session | null>(null)
  const [cargando, setCargando] = useState(true)

  useEffect(() => {
    void supabase.auth.getSession().then(({ data }) => {
      setSession(data.session)
      setCargando(false)
    })
    const { data } = supabase.auth.onAuthStateChange((evento, nueva) => {
      setSession(nueva)
      // Al cambiar de usuario, no deben quedar datos del anterior en cache.
      if (evento === 'SIGNED_OUT' || evento === 'SIGNED_IN') qc.clear()
    })
    return () => data.subscription.unsubscribe()
  }, [qc])

  const value: AuthState = {
    session,
    cargando,
    async signIn(email, password) {
      const { error } = await supabase.auth.signInWithPassword({ email, password })
      if (error) throw error
    },
    async signOut() {
      await supabase.auth.signOut()
    },
  }

  return <AuthContext value={value}>{children}</AuthContext>
}

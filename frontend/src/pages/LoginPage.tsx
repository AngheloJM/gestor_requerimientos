import { type FormEvent, useState } from 'react'
import { Navigate, useLocation } from 'react-router-dom'

import { ApiError } from '@/api/client'
import { useAuth } from '@/auth/useAuth'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Logo } from '@/components/marca/Logo'
import { Label } from '@/components/ui/label'

export function LoginPage() {
  const { autenticado, signIn } = useAuth()
  const location = useLocation()
  const [error, setError] = useState<string | null>(null)
  const [enviando, setEnviando] = useState(false)

  const destino = (location.state as { desde?: string } | null)?.desde ?? '/tablero'
  if (autenticado) return <Navigate to={destino} replace />

  async function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault()
    const form = new FormData(e.currentTarget)
    setEnviando(true)
    setError(null)
    try {
      await signIn(String(form.get('email')), String(form.get('password')))
    } catch (e) {
      setError(
        e instanceof ApiError && e.status === 429
          ? 'Demasiados intentos. Esperá unos minutos antes de volver a intentar.'
          : e instanceof ApiError && e.status === 401
            ? 'Email o contraseña incorrectos.'
            : 'No se pudo conectar con el servidor.',
      )
    } finally {
      setEnviando(false)
    }
  }

  return (
    <div className="grid min-h-svh lg:grid-cols-2">
      <aside className="relative hidden overflow-hidden bg-nova-navy p-12 text-white lg:flex lg:flex-col">
        <Logo claro />
        <div className="mt-auto max-w-md">
          <h1 className="text-4xl leading-tight font-semibold text-white">
            Tecnología que conecta. <span className="text-nova-naranja-suave">Experiencias</span> que transforman.
          </h1>
          <p className="mt-4 text-white/70">
            Requerimientos de personal, matriz de servicios y seguimiento en un solo lugar.
          </p>
        </div>
        <div aria-hidden className="absolute -right-24 -bottom-24 size-80 rounded-full bg-nova-azul/40 blur-3xl" />
      </aside>

      <main className="flex items-center justify-center bg-background p-6">
        <div className="w-full max-w-sm">
          <Logo className="mb-8 lg:hidden" />
          <h2 className="text-2xl font-semibold">Iniciar sesión</h2>
          <p className="mt-1 text-sm text-nova-gris">Ingresá con tu correo corporativo.</p>
          <form onSubmit={onSubmit} className="mt-6 grid gap-4">
            <div className="grid gap-2">
              <Label htmlFor="email">Email</Label>
              <Input id="email" name="email" type="email" autoComplete="email" required className="h-11 bg-white" />
            </div>
            <div className="grid gap-2">
              <Label htmlFor="password">Contraseña</Label>
              <Input
                id="password"
                name="password"
                type="password"
                autoComplete="current-password"
                required
                className="h-11 bg-white"
              />
            </div>
            {error && (
              <Alert variant="destructive">
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}
            <Button type="submit" disabled={enviando} className="h-11 text-base">
              {enviando ? 'Ingresando…' : 'Ingresar'}
            </Button>
          </form>
        </div>
      </main>
    </div>
  )
}

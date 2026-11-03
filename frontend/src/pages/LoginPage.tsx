import { type FormEvent, useState } from 'react'
import { Navigate, useLocation } from 'react-router-dom'

import { ApiError } from '@/api/client'
import { useAuth } from '@/auth/useAuth'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
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
    <div className="flex min-h-svh items-center justify-center p-4">
      <Card className="w-full max-w-sm">
        <CardHeader>
          <CardTitle>Iniciar sesión</CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={onSubmit} className="grid gap-4">
            <div className="grid gap-2">
              <Label htmlFor="email">Email</Label>
              <Input id="email" name="email" type="email" autoComplete="email" required />
            </div>
            <div className="grid gap-2">
              <Label htmlFor="password">Contraseña</Label>
              <Input
                id="password"
                name="password"
                type="password"
                autoComplete="current-password"
                required
              />
            </div>
            {error && (
              <Alert variant="destructive">
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}
            <Button type="submit" disabled={enviando}>
              {enviando ? 'Ingresando…' : 'Ingresar'}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  )
}

import { Navigate, Outlet, useLocation } from 'react-router-dom'

import { useAuth } from '@/auth/useAuth'
import { Skeleton } from '@/components/ui/skeleton'

/** Protege rutas: sin sesión redirige a /login (y vuelve a la ruta original al loguearse). */
export function RequireAuth() {
  const { session, cargando } = useAuth()
  const location = useLocation()

  if (cargando) return <Skeleton className="m-6 h-8 w-48" />
  if (!session) return <Navigate to="/login" replace state={{ desde: location.pathname }} />
  return <Outlet />
}

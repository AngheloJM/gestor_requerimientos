import { NavLink, Outlet } from 'react-router-dom'

import { useMe } from '@/api'
import { useAuth } from '@/auth/useAuth'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'

const LINKS = [
  { to: '/tablero', label: 'Tablero' },
  { to: '/matriz', label: 'Matriz' },
  { to: '/almanaque', label: 'Almanaque' },
  { to: '/aprobaciones', label: 'Aprobaciones', soloRol: 'rrhh_admin' },
] as const

export function Layout() {
  const { signOut } = useAuth()
  const { data: me } = useMe()

  return (
    <div className="min-h-svh bg-background">
      <header className="border-b">
        <nav className="mx-auto flex max-w-6xl flex-wrap items-center gap-2 px-4 py-3">
          <span className="mr-4 font-semibold">Gestor de Requerimientos</span>
          {LINKS.filter((l) => !('soloRol' in l) || me?.rol === l.soloRol).map((l) => (
            <NavLink
              key={l.to}
              to={l.to}
              className={({ isActive }) =>
                cn(
                  'rounded-md px-3 py-1.5 text-sm hover:bg-muted',
                  isActive && 'bg-muted font-medium',
                )
              }
            >
              {l.label}
            </NavLink>
          ))}
          <div className="ml-auto flex items-center gap-2">
            {me && <Badge variant="secondary">{me.rol}</Badge>}
            <Button variant="outline" size="sm" onClick={() => void signOut()}>
              Salir
            </Button>
          </div>
        </nav>
      </header>
      <main className="mx-auto max-w-6xl px-4 py-6">
        <Outlet />
      </main>
    </div>
  )
}

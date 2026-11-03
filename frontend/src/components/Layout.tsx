import {
  CalendarDays,
  ClipboardCheck,
  ClipboardList,
  LayoutDashboard,
  LogOut,
  Menu,
  ShieldCheck,
  Table2,
  X,
} from 'lucide-react'
import { useState } from 'react'
import { NavLink, Outlet, useLocation } from 'react-router-dom'

import { useMe } from '@/api'
import { useAuth } from '@/auth/useAuth'
import { Logo } from '@/components/marca/Logo'
import { cn } from '@/lib/utils'

const LINKS = [
  { to: '/requerimientos', label: 'Mis requerimientos', icono: ClipboardList },
  { to: '/tablero', label: 'Tablero', icono: LayoutDashboard },
  { to: '/matriz', label: 'Matriz de servicio', icono: Table2 },
  { to: '/almanaque', label: 'Almanaque', icono: CalendarDays },
  // Solo se oculta en el menú; la autorización real la aplica RLS en la base.
  { to: '/aprobaciones', label: 'Aprobaciones', icono: ClipboardCheck, soloRol: 'rrhh_admin' },
  { to: '/roles', label: 'Roles y permisos', icono: ShieldCheck, soloRol: 'rrhh_admin' },
] as const

const ROLES: Record<string, string> = {
  rrhh_admin: 'RR.HH. · Administrador',
  encargado_costos: 'Costos',
  estandar: 'Encargado de servicio',
  externo: 'Externo',
}

function iniciales(nombre: string | null | undefined): string {
  const partes = (nombre ?? '?').trim().split(/\s+/)
  return partes.slice(0, 2).map((p) => p[0]?.toUpperCase() ?? '').join('') || '?'
}

function Navegacion({ alNavegar }: { alNavegar?: () => void }) {
  const { data: me } = useMe()
  return (
    <nav className="grid gap-1">
      {LINKS.filter((l) => !('soloRol' in l) || me?.rol === l.soloRol).map(({ to, label, icono: Icono }) => (
        <NavLink
          key={to}
          to={to}
          onClick={alNavegar}
          className={({ isActive }) =>
            cn(
              'flex items-center gap-3 rounded-lg px-3 py-2.5 text-[15px] transition-colors',
              isActive
                ? 'bg-sidebar-primary font-medium text-white shadow-nova-sm'
                : 'text-sidebar-foreground hover:bg-sidebar-accent hover:text-white',
            )
          }
        >
          <Icono className="size-[18px]" aria-hidden />
          {label}
        </NavLink>
      ))}
    </nav>
  )
}

function Usuario() {
  const { signOut } = useAuth()
  const { data: me } = useMe()
  return (
    <div className="flex items-center gap-3 rounded-xl bg-white/5 p-3">
      <span className="grid size-9 shrink-0 place-items-center rounded-full bg-nova-azul-medio text-sm font-semibold text-white">
        {iniciales(me?.nombre)}
      </span>
      <div className="min-w-0 flex-1 leading-tight">
        <p className="truncate text-sm font-medium text-white">{me?.nombre ?? '…'}</p>
        <p className="truncate text-xs text-white/60">{me?.rol ? (ROLES[me.rol] ?? me.rol) : ''}</p>
      </div>
      <button
        type="button"
        onClick={() => void signOut()}
        className="rounded-md p-1.5 text-white/70 hover:bg-white/10 hover:text-white"
        title="Cerrar sesión"
        aria-label="Cerrar sesión"
      >
        <LogOut className="size-4" />
      </button>
    </div>
  )
}

export function Layout() {
  const [abierto, setAbierto] = useState(false)
  const { pathname } = useLocation()
  const titulo =
    LINKS.find((l) => pathname.startsWith(l.to))?.label ??
    (pathname.startsWith('/nuevo-requerimiento') ? 'Nuevo requerimiento' : '')

  const barra = (
    <div className="flex h-full flex-col gap-6 p-4">
      <Logo claro className="px-1 pt-1" />
      <Navegacion alNavegar={() => setAbierto(false)} />
      <div className="mt-auto">
        <Usuario />
      </div>
    </div>
  )

  return (
    <div className="min-h-svh bg-background lg:pl-64">
      {/* Barra lateral fija (escritorio) */}
      <aside className="fixed inset-y-0 left-0 hidden w-64 bg-sidebar lg:block">{barra}</aside>

      {/* Barra lateral desplegable (móvil/tablet) */}
      {abierto && (
        <div className="fixed inset-0 z-40 lg:hidden" role="dialog" aria-modal>
          <button
            type="button"
            className="absolute inset-0 bg-nova-titulo/40"
            aria-label="Cerrar menú"
            onClick={() => setAbierto(false)}
          />
          <aside className="absolute inset-y-0 left-0 w-72 bg-sidebar shadow-nova-lg">
            <button
              type="button"
              onClick={() => setAbierto(false)}
              className="absolute top-4 right-3 rounded-md p-1.5 text-white/70 hover:bg-white/10"
              aria-label="Cerrar menú"
            >
              <X className="size-5" />
            </button>
            {barra}
          </aside>
        </div>
      )}

      <header className="sticky top-0 z-30 flex h-16 items-center gap-3 border-b bg-white/90 px-4 backdrop-blur lg:px-8">
        <button
          type="button"
          onClick={() => setAbierto(true)}
          className="rounded-md p-2 text-nova-titulo hover:bg-muted lg:hidden"
          aria-label="Abrir menú"
        >
          <Menu className="size-5" />
        </button>
        <h1 className="text-xl font-semibold">{titulo}</h1>
      </header>

      <main className="mx-auto max-w-7xl px-4 py-6 lg:px-8">
        <Outlet />
      </main>
    </div>
  )
}

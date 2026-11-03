import { Lock, Plus, ShieldCheck } from 'lucide-react'
import { Link } from 'react-router-dom'

import { usePermisos, useRoles } from '@/api'
import { EstadoBadge } from '@/components/EstadoBadge'
import { ErrorApi } from '@/components/ErrorApi'
import { AvisoFase } from '@/components/roles/AvisoFase'
import { buttonVariants } from '@/components/ui/button'
import { Skeleton } from '@/components/ui/skeleton'
import { agrupar, contar, desdeAsignados } from '@/lib/permisos'

export function RolesPage() {
  const roles = useRoles()
  const catalogo = usePermisos()

  const error = roles.error ?? catalogo.error
  const modulos = catalogo.data ? agrupar(catalogo.data) : []

  return (
    <div className="grid gap-6">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h2 className="text-2xl font-semibold text-nova-titulo">Roles y permisos</h2>
          <p className="mt-1 text-sm text-nova-gris">
            Definí qué puede ver, crear, editar o eliminar cada rol, módulo por módulo.
          </p>
        </div>
        <Link to="/roles/nuevo" className={buttonVariants()}>
          <Plus aria-hidden /> Crear rol
        </Link>
      </div>

      <AvisoFase />

      {error ? (
        <ErrorApi error={error} />
      ) : roles.isPending || catalogo.isPending ? (
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
          {[0, 1, 2].map((i) => (
            <Skeleton key={i} className="h-44 rounded-xl" />
          ))}
        </div>
      ) : (
        <ul className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
          {roles.data!.map((r) => {
            const sel = desdeAsignados(r.permisos)
            return (
              <li key={r.id}>
                <Link
                  to={`/roles/${r.id}`}
                  className="flex h-full flex-col rounded-xl border bg-card p-5 shadow-nova-sm transition-shadow hover:shadow-nova-md focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
                >
                  <div className="flex items-start justify-between gap-3">
                    <span className="grid size-10 shrink-0 place-items-center rounded-lg bg-nova-hielo text-nova-azul">
                      {r.es_sistema ? <Lock className="size-5" aria-hidden /> : <ShieldCheck className="size-5" aria-hidden />}
                    </span>
                    <div className="flex flex-wrap justify-end gap-1.5">
                      {r.es_sistema ? <EstadoBadge>Sistema</EstadoBadge> : <EstadoBadge tono="marca">Personalizado</EstadoBadge>}
                      {!r.activo && <EstadoBadge tono="error">Inactivo</EstadoBadge>}
                    </div>
                  </div>
                  <p className="mt-3 font-semibold text-nova-titulo">{r.nombre}</p>
                  <p className="mt-1 line-clamp-2 min-h-10 text-sm text-nova-gris">{r.descripcion ?? 'Sin descripción'}</p>
                  <p className="mt-3 text-sm font-medium text-nova-texto-2">
                    {r.permisos.length} de {catalogo.data!.length} permisos
                  </p>
                  <ul className="mt-2 flex flex-wrap gap-1.5" aria-label="Permisos por módulo">
                    {modulos.map((m) => {
                      const { marcados, total } = contar(sel, m)
                      if (marcados === 0) return null
                      return (
                        <li key={m.modulo} className="rounded-md bg-nova-hielo px-2 py-0.5 text-xs text-nova-azul">
                          {m.modulo} {marcados}/{total}
                        </li>
                      )
                    })}
                  </ul>
                </Link>
              </li>
            )
          })}
        </ul>
      )}
    </div>
  )
}

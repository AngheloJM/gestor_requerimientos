import { CheckCircle2, Info, Pencil, Search, X } from 'lucide-react'
import { useMemo, useState } from 'react'

import {
  type OpcionesUsuario,
  type Rol,
  type UsuarioAdmin,
  useGuardarUsuario,
  useMe,
  useOpcionesUsuario,
  useUsuarios,
} from '@/api'
import { EstadoBadge } from '@/components/EstadoBadge'
import { ErrorApi } from '@/components/ErrorApi'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import { ETIQUETAS_ROL, etiquetaRol } from '@/lib/roles'
import { cn } from '@/lib/utils'

const selectClase =
  'h-9 w-full rounded-lg border border-input bg-white px-2.5 text-sm outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:opacity-50'

function Editor({
  u,
  usuarios,
  opciones,
  esYo,
  cerrar,
}: {
  u: UsuarioAdmin
  usuarios: UsuarioAdmin[]
  opciones: OpcionesUsuario
  esYo: boolean
  cerrar: () => void
}) {
  const guardar = useGuardarUsuario(u.id)
  const [cargo, setCargo] = useState(u.cargo?.id ?? '')
  const [superior, setSuperior] = useState(u.inmediato_superior?.id ?? '')
  const [division, setDivision] = useState(u.division ?? '')
  const [region, setRegion] = useState(u.region ?? '')
  const [rol, setRol] = useState(u.rol)
  const [activo, setActivo] = useState(u.activo)
  const [servicios, setServicios] = useState<Set<string>>(() => new Set(u.servicios.map((s) => s.id)))
  const [guardado, setGuardado] = useState(false)

  // Servicios activos + los ya asignados que hoy estén inactivos (para no perderlos al guardar).
  const listaServicios = useMemo(() => {
    const ids = new Set(opciones.servicios.map((s) => s.id))
    return [...opciones.servicios, ...u.servicios.filter((s) => !ids.has(s.id))]
  }, [opciones.servicios, u.servicios])

  const marcar = (id: string) => {
    const s = new Set(servicios)
    if (s.has(id)) s.delete(id)
    else s.add(id)
    setServicios(s)
    setGuardado(false)
  }

  const enviar = (e: React.FormEvent) => {
    e.preventDefault()
    guardar.mutate(
      {
        cargo_id: cargo || null,
        inmediato_superior_id: superior || null,
        division: division.trim() || null,
        region: region.trim() || null,
        rol: rol as Rol,
        activo,
        servicios: [...servicios],
      },
      { onSuccess: () => setGuardado(true) },
    )
  }

  const cambio = <T,>(set: (v: T) => void) => (v: T) => {
    set(v)
    setGuardado(false)
  }

  return (
    <form
      onSubmit={enviar}
      className="grid gap-4 rounded-xl border bg-card p-5 shadow-nova-sm lg:sticky lg:top-20"
      aria-label={`Editar usuario ${u.nombre}`}
    >
      <div className="flex items-start justify-between gap-2">
        <div>
          <p className="text-xs text-nova-gris">Editar usuario</p>
          <h3 className="text-lg font-semibold text-nova-titulo">{u.nombre}</h3>
        </div>
        <Button type="button" variant="ghost" size="icon" onClick={cerrar} aria-label="Cerrar">
          <X />
        </Button>
      </div>

      <div className="grid gap-3 sm:grid-cols-2">
        <div className="grid gap-1.5">
          <Label htmlFor="cargo">Cargo</Label>
          <select id="cargo" className={selectClase} value={cargo} onChange={(e) => cambio(setCargo)(e.target.value)}>
            <option value="">Sin cargo</option>
            {opciones.cargos.map((c) => (
              <option key={c.id} value={c.id}>
                {c.nombre}
              </option>
            ))}
          </select>
        </div>
        <div className="grid gap-1.5">
          <Label htmlFor="superior">Inmediato superior</Label>
          <select id="superior" className={selectClase} value={superior} onChange={(e) => cambio(setSuperior)(e.target.value)}>
            <option value="">Sin asignar</option>
            {usuarios
              .filter((x) => x.id !== u.id)
              .map((x) => (
                <option key={x.id} value={x.id}>
                  {x.nombre}
                </option>
              ))}
          </select>
        </div>
        <div className="grid gap-1.5">
          <Label htmlFor="division">División</Label>
          <Input id="division" className="h-9 bg-white" maxLength={120} value={division} onChange={(e) => cambio(setDivision)(e.target.value)} />
        </div>
        <div className="grid gap-1.5">
          <Label htmlFor="region">Región</Label>
          <Input id="region" className="h-9 bg-white" maxLength={120} value={region} onChange={(e) => cambio(setRegion)(e.target.value)} />
        </div>
        <div className="grid gap-1.5">
          <Label htmlFor="rol">Rol</Label>
          <select id="rol" className={selectClase} value={rol} disabled={esYo} onChange={(e) => cambio(setRol)(e.target.value)}>
            {opciones.roles.map((r) => (
              <option key={r} value={r}>
                {ETIQUETAS_ROL[r] ?? r}
              </option>
            ))}
          </select>
        </div>
        <label className="flex items-center gap-2 self-end pb-2 text-sm">
          <input
            type="checkbox"
            className="size-4 accent-nova-azul"
            checked={activo}
            disabled={esYo}
            onChange={(e) => cambio(setActivo)(e.target.checked)}
          />
          Activo
        </label>
      </div>
      {esYo && (
        <p className="flex gap-2 text-xs text-nova-gris">
          <Info className="size-3.5 shrink-0" aria-hidden />
          Tu propio rol y estado los cambia otra persona de RR.HH., para que no pierdas el acceso por error.
        </p>
      )}

      <fieldset className="grid gap-2 border-t pt-4">
        <legend className="text-sm font-semibold text-nova-titulo">Servicios asignados</legend>
        {rol === 'rrhh_admin' ? (
          <p className="rounded-lg bg-nova-hielo p-3 text-sm text-nova-texto-2">
            Este usuario ve todos los servicios automáticamente por su rol; no necesita asignación.
          </p>
        ) : (
          <ul className="grid max-h-64 gap-1 overflow-y-auto rounded-lg border bg-white p-2 sm:grid-cols-2">
            {listaServicios.map((s) => (
              <li key={s.id}>
                <label className="flex cursor-pointer items-center gap-2 rounded px-2 py-1 text-sm hover:bg-nova-hielo/50">
                  <input type="checkbox" className="size-4 accent-nova-azul" checked={servicios.has(s.id)} onChange={() => marcar(s.id)} />
                  {s.nombre}
                </label>
              </li>
            ))}
          </ul>
        )}
      </fieldset>

      {guardar.error && <ErrorApi error={guardar.error} />}
      <div className="flex items-center justify-end gap-3">
        {guardado && !guardar.isPending && (
          <span className="flex items-center gap-1.5 text-sm text-exito" role="status">
            <CheckCircle2 className="size-4" aria-hidden /> Guardado
          </span>
        )}
        <Button type="submit" disabled={guardar.isPending}>
          {guardar.isPending ? 'Guardando…' : 'Guardar usuario'}
        </Button>
      </div>
    </form>
  )
}

export function UsuariosPage() {
  const usuarios = useUsuarios()
  const opciones = useOpcionesUsuario()
  const { data: me } = useMe()
  const [buscar, setBuscar] = useState('')
  const [sel, setSel] = useState<string | null>(null)

  const error = usuarios.error ?? opciones.error
  const lista = (usuarios.data ?? []).filter((u) => {
    const q = buscar.trim().toLowerCase()
    return !q || [u.nombre, u.cargo?.nombre, u.division, u.region, etiquetaRol(u.rol)].some((v) => v?.toLowerCase().includes(q))
  })
  const actual = usuarios.data?.find((u) => u.id === sel)

  return (
    <div className="grid gap-5">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h2 className="text-2xl font-semibold text-nova-titulo">Usuarios</h2>
          <p className="mt-1 text-sm text-nova-gris">Cargo, inmediato superior, rol y servicios de cada persona.</p>
        </div>
        <div className="relative w-full sm:w-72">
          <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-nova-gris" aria-hidden />
          <Input
            aria-label="Buscar usuario"
            placeholder="Buscar usuario…"
            className="h-9 bg-white pl-8"
            value={buscar}
            onChange={(e) => setBuscar(e.target.value)}
          />
        </div>
      </div>

      {error ? (
        <ErrorApi error={error} />
      ) : usuarios.isPending || opciones.isPending ? (
        <Skeleton className="h-64 rounded-xl" />
      ) : (
        <div className={cn('grid items-start gap-5', actual && 'lg:grid-cols-[1fr_30rem]')}>
          <div className="overflow-x-auto rounded-xl border bg-card shadow-nova-sm">
            <table className="w-full min-w-[640px] text-sm">
              <thead className="bg-nova-hielo/60 text-left text-xs tracking-wide text-nova-gris uppercase">
                <tr>
                  {['Nombre', 'Cargo', 'Rol', 'Servicios', 'Estado', ''].map((h) => (
                    <th key={h} className="px-4 py-3 font-medium">
                      {h}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody className="divide-y">
                {lista.map((u) => (
                  <tr key={u.id} className={cn('hover:bg-nova-hielo/40', u.id === sel && 'bg-nova-hielo')}>
                    <td className="px-4 py-3">
                      <p className="font-medium text-nova-titulo">
                        {u.nombre}
                        {u.id === me?.user_id && <span className="ml-2 text-xs font-normal text-nova-naranja">vos</span>}
                      </p>
                      <p className="text-xs text-nova-gris">
                        {[u.division, u.region].filter(Boolean).join(' · ') || 'Sin división ni región'}
                      </p>
                    </td>
                    <td className="px-4 py-3">{u.cargo?.nombre ?? <span className="text-nova-gris">Sin cargo</span>}</td>
                    <td className="px-4 py-3">{etiquetaRol(u.rol)}</td>
                    <td className="px-4 py-3 text-nova-texto-2">
                      {u.rol === 'rrhh_admin' ? 'Todos' : u.servicios.length ? u.servicios.map((s) => s.nombre).join(', ') : '—'}
                    </td>
                    <td className="px-4 py-3">
                      {u.activo ? <EstadoBadge tono="exito">Activo</EstadoBadge> : <EstadoBadge tono="error">Inactivo</EstadoBadge>}
                    </td>
                    <td className="px-4 py-3 text-right">
                      <Button variant="ghost" size="icon" onClick={() => setSel(u.id)} aria-label={`Editar ${u.nombre}`}>
                        <Pencil />
                      </Button>
                    </td>
                  </tr>
                ))}
                {lista.length === 0 && (
                  <tr>
                    <td colSpan={6} className="px-4 py-10 text-center text-nova-gris">
                      Sin resultados.
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
          {actual && (
            <Editor
              key={actual.id}
              u={actual}
              usuarios={usuarios.data!}
              opciones={opciones.data!}
              esYo={actual.id === me?.user_id}
              cerrar={() => setSel(null)}
            />
          )}
        </div>
      )}
    </div>
  )
}

import { ClipboardCheck, FileText, Plus } from 'lucide-react'
import { useMemo, useState } from 'react'
import { Link, useNavigate } from 'react-router-dom'

import { type Requerimiento, useMe, useRequerimientos } from '@/api'
import { ESTADOS_REQUERIMIENTO, EstadoRequerimiento, etiquetaTipo } from '@/components/EstadoRequerimiento'
import { ErrorApi } from '@/components/ErrorApi'
import { buttonVariants } from '@/components/ui/button'
import { Skeleton } from '@/components/ui/skeleton'
import { formatFecha } from '@/lib/format'
import { cn } from '@/lib/utils'

const texto = (v: string | null | undefined) => v ?? '—'
const folio = (r: Requerimiento) => r.folio ?? `#${r.numero}`
const creado = (r: Requerimiento) => formatFecha(r.creado.slice(0, 10))

function Segmento<T extends string>({
  valor,
  opciones,
  onChange,
  etiqueta,
}: {
  valor: T
  opciones: { valor: T; texto: string }[]
  onChange: (v: T) => void
  etiqueta: string
}) {
  return (
    <div role="radiogroup" aria-label={etiqueta} className="inline-flex rounded-lg border bg-white p-1 shadow-nova-xs">
      {opciones.map((o) => (
        <button
          key={o.valor}
          type="button"
          role="radio"
          aria-checked={valor === o.valor}
          onClick={() => onChange(o.valor)}
          className={cn(
            'rounded-md px-3 py-1.5 text-sm transition-colors',
            valor === o.valor ? 'bg-nova-azul font-medium text-white' : 'text-nova-texto-2 hover:bg-nova-hielo',
          )}
        >
          {o.texto}
        </button>
      ))}
    </div>
  )
}

function Tabla({ filas }: { filas: Requerimiento[] }) {
  const navegar = useNavigate()
  return (
    <div className="hidden overflow-x-auto rounded-xl border bg-card shadow-nova-sm md:block">
      <table className="w-full text-sm">
        <thead className="bg-nova-hielo/60 text-left text-xs tracking-wide text-nova-gris uppercase">
          <tr>
            {['Folio', 'Servicio', 'Cargo', 'Tipo', 'Cant.', 'Inicio de operación', 'Creado', 'Estado'].map((h) => (
              <th key={h} className="px-4 py-3 font-medium">
                {h}
              </th>
            ))}
          </tr>
        </thead>
        <tbody className="divide-y">
          {filas.map((r) => (
            <tr key={r.id} className="cursor-pointer hover:bg-nova-hielo/40" onClick={() => navegar(`/requerimientos/${r.id}`)}>
              <td className="px-4 py-3 font-medium whitespace-nowrap text-nova-titulo">
                <Link to={`/requerimientos/${r.id}`} className="hover:text-nova-azul hover:underline" onClick={(e) => e.stopPropagation()}>
                  {folio(r)}
                </Link>
                {r.es_mio && <span className="ml-2 text-xs font-normal text-nova-naranja">mío</span>}
              </td>
              <td className="px-4 py-3">{texto(r.servicio?.nombre)}</td>
              <td className="px-4 py-3">{texto(r.cargo.nombre)}</td>
              <td className="px-4 py-3">{etiquetaTipo(r.tipo)}</td>
              <td className="px-4 py-3 tabular-nums">{r.cantidad ?? '—'}</td>
              <td className="px-4 py-3 whitespace-nowrap">
                {r.fecha_inicio_operacion ? formatFecha(r.fecha_inicio_operacion) : <span className="text-nova-gris">Sin informar</span>}
              </td>
              <td className="px-4 py-3 whitespace-nowrap">{creado(r)}</td>
              <td className="px-4 py-3">
                <EstadoRequerimiento valor={r.estado} />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

function Tarjetas({ filas }: { filas: Requerimiento[] }) {
  return (
    <ul className="grid gap-3 md:hidden">
      {filas.map((r) => (
        <li key={r.id} className="rounded-xl border bg-card p-4 shadow-nova-sm">
          <div className="flex items-start justify-between gap-2">
            <div>
              <Link to={`/requerimientos/${r.id}`} className="font-semibold text-nova-titulo hover:text-nova-azul hover:underline">
                {folio(r)}
              </Link>
              <p className="text-sm text-nova-texto-2">
                {texto(r.servicio?.nombre)} · {texto(r.cargo.nombre)}
              </p>
            </div>
            <EstadoRequerimiento valor={r.estado} />
          </div>
          <dl className="mt-3 grid grid-cols-2 gap-x-4 gap-y-1 text-sm">
            <dt className="text-nova-gris">Tipo</dt>
            <dd>{etiquetaTipo(r.tipo)}</dd>
            <dt className="text-nova-gris">Cantidad</dt>
            <dd>{r.cantidad ?? '—'}</dd>
            <dt className="text-nova-gris">Inicio de operación</dt>
            <dd>{r.fecha_inicio_operacion ? formatFecha(r.fecha_inicio_operacion) : 'Sin informar'}</dd>
            <dt className="text-nova-gris">Creado</dt>
            <dd>{creado(r)}</dd>
          </dl>
        </li>
      ))}
    </ul>
  )
}

export function MisRequerimientosPage() {
  const { data: me } = useMe()
  const esRrhh = me?.rol === 'rrhh_admin'
  // RR.HH. arranca viendo todos; el resto, los suyos.
  const [alcance, setAlcance] = useState<'mios' | 'todos' | null>(null)
  const vista = alcance ?? (esRrhh ? 'todos' : 'mios')
  const [estado, setEstado] = useState<string>('todos')
  const { data, isPending, error } = useRequerimientos(vista === 'mios')

  const conteo = useMemo(() => {
    const c: Record<string, number> = {}
    for (const r of data ?? []) c[r.estado ?? ''] = (c[r.estado ?? ''] ?? 0) + 1
    return c
  }, [data])
  const filas = (data ?? []).filter((r) => estado === 'todos' || r.estado === estado)

  return (
    <div className="grid gap-6">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h2 className="text-2xl font-semibold text-nova-titulo">
            {vista === 'todos' ? 'Requerimientos' : 'Mis requerimientos'}
          </h2>
          <p className="mt-1 text-sm text-nova-gris">
            {vista === 'todos'
              ? esRrhh
                ? 'Como RR.HH. ves los requerimientos de todos los usuarios.'
                : 'Los que pediste y los de los servicios que tenés asignados.'
              : 'Los que pediste, con su estado de aprobación.'}
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          {esRrhh && (
            <Link to="/aprobaciones" className={buttonVariants({ variant: 'outline' })}>
              <ClipboardCheck aria-hidden /> Aprobación de requerimientos
            </Link>
          )}
          <Link to="/nuevo-requerimiento" className={buttonVariants()}>
            <Plus aria-hidden /> Nuevo requerimiento
          </Link>
        </div>
      </div>

      <div className="flex flex-wrap items-center gap-3">
        <Segmento
          etiqueta="Alcance"
          valor={vista}
          onChange={setAlcance}
          opciones={[
            { valor: 'mios', texto: 'Míos' },
            { valor: 'todos', texto: 'Todos los visibles' },
          ]}
        />
        <div className="flex flex-wrap gap-2" role="group" aria-label="Filtrar por estado">
          {[['todos', 'Todos'] as const, ...Object.entries(ESTADOS_REQUERIMIENTO).map(([k, v]) => [k, v.etiqueta] as const)].map(
            ([k, etiqueta]) => {
              const n = k === 'todos' ? (data?.length ?? 0) : (conteo[k] ?? 0)
              if (k !== 'todos' && n === 0) return null
              return (
                <button
                  key={k}
                  type="button"
                  aria-pressed={estado === k}
                  onClick={() => setEstado(k)}
                  className={cn(
                    'rounded-full border px-3 py-1 text-sm transition-colors',
                    estado === k
                      ? 'border-nova-azul bg-nova-hielo font-medium text-nova-azul'
                      : 'bg-white text-nova-texto-2 hover:bg-nova-hielo',
                  )}
                >
                  {etiqueta} <span className="tabular-nums text-nova-gris">{n}</span>
                </button>
              )
            },
          )}
        </div>
      </div>

      {error ? (
        <ErrorApi error={error} />
      ) : isPending ? (
        <div className="grid gap-3">
          {[0, 1, 2].map((i) => (
            <Skeleton key={i} className="h-14 rounded-xl" />
          ))}
        </div>
      ) : filas.length === 0 ? (
        <div className="grid place-items-center rounded-xl border border-dashed bg-card px-6 py-14 text-center">
          <span className="grid size-12 place-items-center rounded-xl bg-nova-hielo text-nova-azul">
            <FileText className="size-6" aria-hidden />
          </span>
          <p className="mt-4 font-medium text-nova-titulo">
            {vista === 'mios' ? 'Todavía no pediste requerimientos.' : 'No hay requerimientos para mostrar.'}
          </p>
          <p className="mt-1 text-sm text-nova-gris">Cuando crees uno, vas a ver aquí si fue aprobado o no.</p>
        </div>
      ) : (
        <>
          <Tabla filas={filas} />
          <Tarjetas filas={filas} />
        </>
      )}
    </div>
  )
}

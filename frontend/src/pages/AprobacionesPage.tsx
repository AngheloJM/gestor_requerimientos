import { Check, History, X } from 'lucide-react'
import { useMemo, useState } from 'react'

import {
  ApiError,
  type Aprobacion,
  type EstadoRequerimiento as Estado,
  useAprobaciones,
  useEditarRequerimiento,
  useHistorialAprobacion,
} from '@/api'
import { ErrorApi } from '@/components/ErrorApi'
import { ESTADOS_REQUERIMIENTO, EstadoRequerimiento, etiquetaTipo } from '@/components/EstadoRequerimiento'
import { Button } from '@/components/ui/button'
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import { formatFecha } from '@/lib/format'
import { cn } from '@/lib/utils'

const PESTANAS: { valor: string; texto: string }[] = [
  { valor: 'pendiente', texto: 'Pendientes' },
  { valor: 'en_proceso', texto: 'En proceso' },
  { valor: 'aprobado', texto: 'Aprobados' },
  { valor: 'rechazado', texto: 'Rechazados' },
  { valor: 'cerrado', texto: 'Cerrados' },
  { valor: 'todos', texto: 'Todos' },
]

const folio = (a: Aprobacion) => a.folio ?? `#${a.numero}`
const texto = (v: string | null | undefined) => v ?? '—'

function Dato({ etiqueta, children }: { etiqueta: string; children: React.ReactNode }) {
  return (
    <>
      <dt className="text-nova-gris">{etiqueta}</dt>
      <dd className="text-nova-titulo">{children}</dd>
    </>
  )
}

function Historial({ id }: { id: string }) {
  const { data, isPending, error } = useHistorialAprobacion(id)
  if (error) return <ErrorApi error={error} />
  if (isPending) return <Skeleton className="h-16" />
  if (!data.length) return <p className="text-sm text-nova-gris">Sin cambios registrados.</p>
  return (
    <ol className="grid gap-2">
      {[...data].reverse().map((h) => (
        <li key={h.id} className="rounded-lg border bg-white p-2.5 text-sm">
          <p className="text-nova-titulo">
            <span className="font-medium">{h.campo}</span>: {texto(h.valor_anterior)} → {texto(h.valor_nuevo)}
          </p>
          {h.glosa && <p className="mt-0.5 text-nova-texto-2">“{h.glosa}”</p>}
          <p className="mt-0.5 text-xs text-nova-gris">{formatFecha(h.created_at.slice(0, 10))}</p>
        </li>
      ))}
    </ol>
  )
}

function Decision({ a, cerrar }: { a: Aprobacion; cerrar: () => void }) {
  const editar = useEditarRequerimiento()
  const aprobaciones = useAprobaciones()
  const [glosa, setGlosa] = useState('')
  const [otro, setOtro] = useState<Estado | ''>('')
  const [error, setError] = useState<string>()
  const [hecho, setHecho] = useState<string>()

  const decidir = (estado: Estado) => {
    if (glosa.trim().length < 3) {
      setError('Escribí una glosa (al menos 3 caracteres): queda en el historial.')
      return
    }
    setError(undefined)
    setHecho(undefined)
    editar.mutate(
      { id: a.id, estado, version: a.version, glosa: glosa.trim() },
      {
        onSuccess: () => {
          setGlosa('')
          setOtro('')
          setHecho(`Estado cambiado a “${ESTADOS_REQUERIMIENTO[estado]?.etiqueta ?? estado}”.`)
        },
      },
    )
  }

  const conflicto = editar.error instanceof ApiError && editar.error.status === 409
  const otros = (Object.keys(ESTADOS_REQUERIMIENTO) as Estado[]).filter(
    (e) => e !== a.estado && e !== 'aprobado' && e !== 'rechazado',
  )

  return (
    <aside className="grid gap-4 rounded-xl border bg-card p-5 shadow-nova-sm lg:sticky lg:top-20" aria-label="Decisión">
      <div className="flex items-start justify-between gap-2">
        <div>
          <h3 className="text-lg font-semibold text-nova-titulo">{folio(a)}</h3>
          <EstadoRequerimiento valor={a.estado} />
        </div>
        <Button variant="ghost" size="icon" onClick={cerrar} aria-label="Cerrar">
          <X />
        </Button>
      </div>

      <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-sm">
        <Dato etiqueta="Solicitante">{texto(a.solicitante)}</Dato>
        <Dato etiqueta="Servicio">{texto(a.servicio?.nombre)}</Dato>
        <Dato etiqueta="Cargo">{texto(a.cargo.nombre)}</Dato>
        <Dato etiqueta="Tipo">{etiquetaTipo(a.tipo)}</Dato>
        <Dato etiqueta="Motivo">{texto(a.motivo)}</Dato>
        <Dato etiqueta="Cantidad">{a.cantidad ?? '—'}</Dato>
        <Dato etiqueta="Inicio de capacitación">{formatFecha(a.fecha_inicio_capacitacion)}</Dato>
        <Dato etiqueta="Inicio de operación">{formatFecha(a.fecha_inicio_operacion)}</Dato>
      </dl>

      <div className="grid gap-2 border-t pt-4">
        <Label htmlFor="glosa">Glosa (obligatoria)</Label>
        <textarea
          id="glosa"
          rows={3}
          maxLength={2000}
          value={glosa}
          onChange={(e) => setGlosa(e.target.value)}
          aria-invalid={!!error}
          placeholder="Motivo de la decisión"
          className="w-full rounded-lg border border-input bg-white px-2.5 py-2 text-sm outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 aria-invalid:border-destructive"
        />
        {error && (
          <p className="text-xs text-error" role="alert">
            {error}
          </p>
        )}
        <div className="flex flex-wrap gap-2">
          <Button
            type="button"
            className="bg-exito text-white hover:bg-exito/90"
            disabled={editar.isPending || a.estado === 'aprobado'}
            onClick={() => decidir('aprobado')}
          >
            <Check aria-hidden /> Aprobar
          </Button>
          <Button
            type="button"
            variant="destructive"
            disabled={editar.isPending || a.estado === 'rechazado'}
            onClick={() => decidir('rechazado')}
          >
            <X aria-hidden /> Rechazar
          </Button>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <select
            aria-label="Otro estado"
            value={otro}
            onChange={(e) => setOtro(e.target.value as Estado)}
            className="h-8 rounded-md border border-input bg-white px-2 text-sm"
          >
            <option value="">Otro estado…</option>
            {otros.map((e) => (
              <option key={e} value={e}>
                {ESTADOS_REQUERIMIENTO[e].etiqueta}
              </option>
            ))}
          </select>
          <Button type="button" size="sm" variant="outline" disabled={!otro || editar.isPending} onClick={() => otro && decidir(otro)}>
            Aplicar
          </Button>
        </div>
        {hecho && (
          <p className="text-sm text-exito" role="status">
            {hecho}
          </p>
        )}
        {conflicto ? (
          <div className="rounded-lg border border-advertencia/40 bg-advertencia/10 p-3 text-sm">
            Otra persona modificó este requerimiento mientras lo revisabas.{' '}
            <button type="button" className="font-medium text-nova-azul underline" onClick={() => void aprobaciones.refetch().then(() => editar.reset())}>
              Recargar datos
            </button>
          </div>
        ) : (
          editar.error && <ErrorApi error={editar.error} />
        )}
      </div>

      <div className="grid gap-2 border-t pt-4">
        <h4 className="flex items-center gap-2 text-sm font-semibold text-nova-titulo">
          <History className="size-4" aria-hidden /> Historial
        </h4>
        <Historial id={a.id} />
      </div>
    </aside>
  )
}

export function AprobacionesPage() {
  const { data, isPending, error } = useAprobaciones()
  const [pestana, setPestana] = useState('pendiente')
  const [sel, setSel] = useState<string | null>(null)

  const conteo = useMemo(() => {
    const c: Record<string, number> = {}
    for (const a of data ?? []) c[a.estado ?? ''] = (c[a.estado ?? ''] ?? 0) + 1
    return c
  }, [data])
  const filas = (data ?? []).filter((a) => pestana === 'todos' || a.estado === pestana)
  const actual = data?.find((a) => a.id === sel)

  return (
    <div className="grid gap-5">
      <div>
        <h2 className="text-2xl font-semibold text-nova-titulo">Aprobación de requerimientos</h2>
        <p className="mt-1 text-sm text-nova-gris">
          Aprobá o rechazá cada pedido con una glosa; el solicitante ve el resultado en “Mis requerimientos”.
        </p>
      </div>

      <div role="tablist" aria-label="Estado" className="flex flex-wrap gap-2">
        {PESTANAS.map((p) => (
          <button
            key={p.valor}
            role="tab"
            type="button"
            aria-selected={pestana === p.valor}
            onClick={() => {
              setPestana(p.valor)
              setSel(null)
            }}
            className={cn(
              'rounded-full border px-3 py-1 text-sm transition-colors',
              pestana === p.valor ? 'border-nova-azul bg-nova-hielo font-medium text-nova-azul' : 'bg-white text-nova-texto-2 hover:bg-nova-hielo',
            )}
          >
            {p.texto}{' '}
            <span className="tabular-nums text-nova-gris">{p.valor === 'todos' ? (data?.length ?? 0) : (conteo[p.valor] ?? 0)}</span>
          </button>
        ))}
      </div>

      {error ? (
        <ErrorApi error={error} />
      ) : isPending ? (
        <Skeleton className="h-64 rounded-xl" />
      ) : (
        <div className={cn('grid items-start gap-5', actual && 'lg:grid-cols-[1fr_24rem]')}>
          {filas.length === 0 ? (
            <div className="rounded-xl border border-dashed bg-card p-10 text-center text-sm text-nova-gris">
              No hay requerimientos en este estado.
            </div>
          ) : (
            <ul className="grid gap-2">
              {filas.map((a) => (
                <li key={a.id}>
                  <button
                    type="button"
                    onClick={() => setSel(a.id === sel ? null : a.id)}
                    aria-pressed={a.id === sel}
                    className={cn(
                      'grid w-full gap-1 rounded-xl border bg-card p-4 text-left shadow-nova-xs transition-shadow hover:shadow-nova-md sm:grid-cols-[8rem_1fr_auto] sm:items-center sm:gap-4',
                      a.id === sel && 'border-nova-azul ring-1 ring-nova-azul',
                    )}
                  >
                    <span className="font-semibold text-nova-titulo">{folio(a)}</span>
                    <span className="text-sm text-nova-texto-2">
                      <span className="font-medium text-nova-titulo">{texto(a.cargo.nombre)}</span> ·{' '}
                      {texto(a.servicio?.nombre)} · {a.cantidad ?? '—'} persona(s)
                      <span className="block text-xs text-nova-gris">
                        {texto(a.solicitante)} · inicio {formatFecha(a.fecha_inicio_operacion)}
                      </span>
                    </span>
                    <EstadoRequerimiento valor={a.estado} />
                  </button>
                </li>
              ))}
            </ul>
          )}
          {actual && <Decision key={actual.id} a={actual} cerrar={() => setSel(null)} />}
        </div>
      )}
    </div>
  )
}

import { ChevronLeft, ChevronRight, X } from 'lucide-react'
import { useState } from 'react'
import { useSearchParams } from 'react-router-dom'

import { type DiaAlmanaque, type LineaAlmanaque, useAlmanaque } from '@/api'
import { EstadoBadge, type Tono } from '@/components/EstadoBadge'
import { ErrorApi } from '@/components/ErrorApi'
import { Button } from '@/components/ui/button'
import { Skeleton } from '@/components/ui/skeleton'
import { diaSemanaLunes, formatFecha, nombreMes } from '@/lib/format'
import { cn } from '@/lib/utils'

type Categoria = 'entradas' | 'salidas' | 'entregas' | 'rectificaciones'

const CATEGORIAS: { key: Categoria; titulo: string; verbo: string; chip: string; punto: string }[] = [
  { key: 'entradas', titulo: 'Entradas', verbo: 'Entra', chip: 'bg-exito/10 text-[#0F7A33]', punto: 'bg-exito' },
  { key: 'salidas', titulo: 'Salidas', verbo: 'Sale', chip: 'bg-nova-naranja-claro text-[#B85F00]', punto: 'bg-nova-naranja' },
  { key: 'entregas', titulo: 'Entregas', verbo: 'Entrega', chip: 'bg-nova-hielo text-nova-azul', punto: 'bg-nova-azul' },
  {
    key: 'rectificaciones',
    titulo: 'Rectificaciones',
    verbo: 'Rectifica',
    chip: 'bg-muted text-nova-texto-2',
    punto: 'bg-nova-gris',
  },
]

const ESTADO_ENTREGA: Record<string, { texto: string; tono: Tono }> = {
  pendiente: { texto: 'Pendiente', tono: 'advertencia' },
  aceptada: { texto: 'Aceptada', tono: 'exito' },
  rechazada: { texto: 'Rechazada', tono: 'error' },
}

const SEMANA = ['Lun', 'Mar', 'Mié', 'Jue', 'Vie', 'Sáb', 'Dom']

const cantidadTexto = (l: LineaAlmanaque) =>
  l.registros_sin_cantidad === 0
    ? String(l.cantidad)
    : l.cantidad === 0
      ? 'sin cantidad'
      : `${l.cantidad} (+${l.registros_sin_cantidad} sin cantidad)`

const puesto = (l: LineaAlmanaque) => `${l.cargo ?? 'Cargo'}${l.servicio ? ` · ${l.servicio}` : ' · sin servicio'}`

const total = (d: DiaAlmanaque) => CATEGORIAS.reduce((n, c) => n + d[c.key].length, 0)

function hoyIso(): string {
  const h = new Date()
  return `${h.getFullYear()}-${String(h.getMonth() + 1).padStart(2, '0')}-${String(h.getDate()).padStart(2, '0')}`
}

function Chips({ dia, max }: { dia: DiaAlmanaque; max: number }) {
  const lineas = CATEGORIAS.flatMap((c) => dia[c.key].map((l) => ({ c, l })))
  return (
    <ul className="mt-1 grid gap-1">
      {lineas.slice(0, max).map(({ c, l }, i) => (
        <li key={i} className={cn('truncate rounded px-1.5 py-0.5 text-[11px] leading-tight', c.chip)} title={`${c.verbo}: ${cantidadTexto(l)} ${puesto(l)}`}>
          <span className="font-semibold">
            {c.verbo}: {cantidadTexto(l)}
          </span>{' '}
          {l.cargo}
        </li>
      ))}
      {lineas.length > max && <li className="px-1.5 text-[11px] text-nova-gris">+{lineas.length - max} más</li>}
    </ul>
  )
}

function Detalle({ dia, cerrar }: { dia: DiaAlmanaque; cerrar: () => void }) {
  return (
    <aside className="rounded-xl border bg-card p-5 shadow-nova-sm lg:sticky lg:top-20" aria-label="Detalle del día">
      <div className="flex items-start justify-between gap-2">
        <h3 className="text-lg font-semibold text-nova-titulo">{formatFecha(dia.fecha)}</h3>
        <Button variant="ghost" size="icon" onClick={cerrar} aria-label="Cerrar detalle">
          <X />
        </Button>
      </div>
      {total(dia) === 0 ? (
        <p className="mt-3 text-sm text-nova-gris">Sin movimientos este día.</p>
      ) : (
        <div className="mt-3 grid gap-5">
          {CATEGORIAS.filter((c) => dia[c.key].length > 0).map((c) => (
            <section key={c.key}>
              <h4 className="flex items-center gap-2 text-sm font-semibold text-nova-titulo">
                <span className={cn('size-2.5 rounded-full', c.punto)} aria-hidden />
                {c.titulo} · {dia[c.key].length}
              </h4>
              <ul className="mt-2 grid gap-2">
                {dia[c.key].map((l, i) => {
                  const estado = l.estado_entrega ? ESTADO_ENTREGA[l.estado_entrega] : undefined
                  return (
                    <li key={i} className="rounded-lg border bg-white p-3 text-sm">
                      <div className="flex items-start justify-between gap-2">
                        <p className="font-medium text-nova-titulo">{puesto(l)}</p>
                        <span className="font-semibold tabular-nums text-nova-titulo">{cantidadTexto(l)}</span>
                      </div>
                      <p className="text-nova-gris">{l.cliente ?? 'Sin cliente'}</p>
                      <div className="mt-2 flex flex-wrap items-center gap-1.5">
                        {l.estado_entrega && (
                          <EstadoBadge tono={estado?.tono ?? 'neutral'}>{estado?.texto ?? l.estado_entrega}</EstadoBadge>
                        )}
                        {l.folios.map((f) => (
                          <span key={f} className="rounded bg-nova-hielo px-1.5 py-0.5 text-xs text-nova-azul">
                            {f}
                          </span>
                        ))}
                      </div>
                      {l.registros_sin_cantidad > 0 && (
                        <p className="mt-1 text-xs text-nova-gris">{l.registros_sin_cantidad} registro(s) sin cantidad informada (no se suman).</p>
                      )}
                    </li>
                  )
                })}
              </ul>
            </section>
          ))}
        </div>
      )}
    </aside>
  )
}

export function AlmanaquePage() {
  const [params, setParams] = useSearchParams()
  const ahora = new Date()
  const anio = Number(params.get('anio')) || ahora.getFullYear()
  const mes = Number(params.get('mes')) || ahora.getMonth() + 1
  const [seleccion, setSeleccion] = useState<string | null>(null)
  const { data, isPending, error, isFetching } = useAlmanaque(anio, mes)

  const mover = (delta: number) => {
    const m = mes + delta
    const nuevo = { anio: anio + Math.floor((m - 1) / 12), mes: ((m - 1 + 120) % 12) + 1 }
    setSeleccion(null)
    setParams({ anio: String(nuevo.anio), mes: String(nuevo.mes) }, { replace: true })
  }

  const hoy = hoyIso()
  const dias = data?.dias ?? []
  const diaSel = dias.find((d) => d.fecha === seleccion)
  const vacios = dias.length ? diaSemanaLunes(dias[0].fecha) : 0

  return (
    <div className="grid gap-5">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h2 className="text-2xl font-semibold text-nova-titulo">Almanaque</h2>
          <p className="mt-1 text-sm text-nova-gris">Entradas, salidas y entregas de personal por día.</p>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="outline" size="icon" onClick={() => mover(-1)} aria-label="Mes anterior">
            <ChevronLeft />
          </Button>
          <p className={cn('min-w-40 text-center font-semibold text-nova-titulo', isFetching && 'opacity-60')} aria-live="polite">
            {nombreMes(anio, mes)}
          </p>
          <Button variant="outline" size="icon" onClick={() => mover(1)} aria-label="Mes siguiente">
            <ChevronRight />
          </Button>
        </div>
      </div>

      <ul className="flex flex-wrap gap-x-5 gap-y-2 text-sm text-nova-texto-2" aria-label="Leyenda">
        {CATEGORIAS.map((c) => (
          <li key={c.key} className="flex items-center gap-2">
            <span className={cn('size-2.5 rounded-full', c.punto)} aria-hidden />
            {c.titulo}
          </li>
        ))}
      </ul>

      {error ? (
        <ErrorApi error={error} />
      ) : isPending ? (
        <Skeleton className="h-[520px] rounded-xl" />
      ) : (
        <div className={cn('grid items-start gap-5', diaSel && 'lg:grid-cols-[1fr_22rem]')}>
          {/* Calendario (tablet y escritorio) */}
          <div className="hidden overflow-hidden rounded-xl border bg-card shadow-nova-sm md:block">
            <div className="grid grid-cols-7 border-b bg-nova-hielo/50 text-center text-xs font-medium tracking-wide text-nova-gris uppercase">
              {SEMANA.map((s) => (
                <div key={s} className="py-2">
                  {s}
                </div>
              ))}
            </div>
            <div className="grid grid-cols-7">
              {Array.from({ length: vacios }, (_, i) => (
                <div key={`v${i}`} className="min-h-28 border-r border-b bg-muted/30" />
              ))}
              {dias.map((d) => {
                const n = total(d)
                return (
                  <button
                    key={d.fecha}
                    type="button"
                    onClick={() => setSeleccion(d.fecha === seleccion ? null : d.fecha)}
                    aria-pressed={d.fecha === seleccion}
                    aria-label={`${formatFecha(d.fecha)}: ${n} movimiento(s)`}
                    className={cn(
                      'flex min-h-28 min-w-0 flex-col items-stretch justify-start border-r border-b p-1.5 text-left transition-colors hover:bg-nova-hielo/40 focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none focus-visible:ring-inset',
                      d.fecha === seleccion && 'bg-nova-hielo',
                    )}
                  >
                    <span
                      className={cn(
                        'inline-grid size-6 place-items-center rounded-full text-xs font-medium text-nova-texto-2',
                        d.fecha === hoy && 'bg-nova-azul text-white',
                      )}
                    >
                      {Number(d.fecha.slice(8))}
                    </span>
                    {n > 0 && <Chips dia={d} max={3} />}
                  </button>
                )
              })}
            </div>
          </div>

          {/* Lista (móvil): solo días con movimientos */}
          <ul className="grid gap-2 md:hidden">
            {dias.filter((d) => total(d) > 0).length === 0 && (
              <li className="rounded-xl border border-dashed bg-card p-8 text-center text-sm text-nova-gris">
                Sin movimientos este mes.
              </li>
            )}
            {dias
              .filter((d) => total(d) > 0)
              .map((d) => (
                <li key={d.fecha}>
                  <button
                    type="button"
                    onClick={() => setSeleccion(d.fecha)}
                    className="w-full rounded-xl border bg-card p-3 text-left shadow-nova-xs"
                  >
                    <p className="font-semibold text-nova-titulo">{formatFecha(d.fecha)}</p>
                    <Chips dia={d} max={6} />
                  </button>
                </li>
              ))}
          </ul>

          {diaSel && <Detalle dia={diaSel} cerrar={() => setSeleccion(null)} />}
        </div>
      )}
    </div>
  )
}

import { ChevronLeft, ChevronRight, X } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { useSearchParams } from 'react-router-dom'

import { type DiaAlmanaque, type LineaAlmanaque, type Requerimiento, useAlmanaque, useRequerimientos } from '@/api'
import { EstadoBadge, type Tono } from '@/components/EstadoBadge'
import { ErrorApi } from '@/components/ErrorApi'
import { Button } from '@/components/ui/button'
import { Skeleton } from '@/components/ui/skeleton'
import { diaSemanaLunes, formatFecha, nombreMes } from '@/lib/format'
import { cn } from '@/lib/utils'

/**
 * Revisión 04/11/2026: el calendario muestra, en la **fecha de entrega**, una línea simple
 * "Cargo, Servicio, cantidad" — sin categorías ni colores por tipo de movimiento. El detalle
 * (cliente, estado, folios, fecha pactada de ingreso a operaciones) se ve al tocar el día.
 * Revisión 06/11/2026: una rectificación se muestra como la cantidad que **queda** ("quedan N"),
 * sin la palabra "corrección" ni signo. La API ya trae lo que queda: entrega + rectificaciones
 * (criterio de suma, igual que "reclutados informados"; semántica a validar, ver PENDIENTES).
 * Los ingresos a operación y salidas ya no se muestran en el calendario.
 */
type Linea = LineaAlmanaque & { correccion: boolean }

const ESTADO_ENTREGA: Record<string, { texto: string; tono: Tono }> = {
  pendiente: { texto: 'Pendiente de aceptación', tono: 'advertencia' },
  aceptada: { texto: 'Aceptada', tono: 'exito' },
  rechazada: { texto: 'Rechazada', tono: 'error' },
}

const SEMANA = ['Lun', 'Mar', 'Mié', 'Jue', 'Vie', 'Sáb', 'Dom']
const SEMANA_LARGA = ['lunes', 'martes', 'miércoles', 'jueves', 'viernes', 'sábado', 'domingo']

const lineas = (d: DiaAlmanaque): Linea[] => [
  ...d.entregas.map((l) => ({ ...l, correccion: false })),
  ...d.rectificaciones.map((l) => ({ ...l, correccion: true })),
]

const cantidadTexto = (l: Linea) => {
  const valor = String(l.cantidad).replace('-', '−')
  const n = l.correccion ? `quedan ${valor}` : valor
  if (l.registros_sin_cantidad === 0) return n
  return l.cantidad === 0 ? 'sin cantidad' : `${n} (+${l.registros_sin_cantidad} sin cantidad)`
}

/** "Operador Call Center, Fidelización, 5" — o, si se rectificó, "…, quedan 2". */
const resumen = (l: Linea) => `${l.cargo ?? 'Sin cargo'}, ${l.servicio ?? 'sin servicio'}, ${cantidadTexto(l)}`

function hoyIso(): string {
  const h = new Date()
  return `${h.getFullYear()}-${String(h.getMonth() + 1).padStart(2, '0')}-${String(h.getDate()).padStart(2, '0')}`
}

function Lineas({ dia, max }: { dia: DiaAlmanaque; max: number }) {
  const ls = lineas(dia)
  return (
    <ul className="mt-1 grid gap-1">
      {ls.slice(0, max).map((l, i) => (
        <li key={i} className="rounded bg-muted px-1.5 py-1 text-sm leading-snug [overflow-wrap:anywhere] text-nova-titulo">
          {resumen(l)}
        </li>
      ))}
      {ls.length > max && <li className="px-1.5 text-sm text-nova-gris">+{ls.length - max} más</li>}
    </ul>
  )
}

function Detalle({ dia, cerrar, porFolio }: { dia: DiaAlmanaque; cerrar: () => void; porFolio: Map<string, Requerimiento> }) {
  const ls = lineas(dia)
  // Al elegir un día, el detalle (debajo del calendario) se trae a la vista y recibe el foco.
  const ref = useRef<HTMLElement>(null)
  useEffect(() => {
    ref.current?.scrollIntoView({ behavior: 'smooth', block: 'start' })
    ref.current?.focus({ preventScroll: true })
  }, [dia.fecha])
  return (
    <aside ref={ref} tabIndex={-1} className="scroll-mt-20 rounded-xl border bg-card p-5 shadow-nova-sm outline-none" aria-label="Detalle del día">
      <div className="flex items-start justify-between gap-2">
        <h3 className="text-xl font-semibold text-nova-titulo">
          <span className="block text-sm font-medium text-nova-gris capitalize">{SEMANA_LARGA[diaSemanaLunes(dia.fecha)]}</span>
          {formatFecha(dia.fecha)}
        </h3>
        <Button variant="ghost" size="icon" onClick={cerrar} aria-label="Cerrar detalle">
          <X />
        </Button>
      </div>
      {ls.length === 0 ? (
        <p className="mt-3 text-base text-nova-gris">Sin entregas este día.</p>
      ) : (
        <ul className="mt-4 grid gap-3 md:grid-cols-2 xl:grid-cols-3">
          {ls.map((l, i) => {
            const estado = l.estado_entrega ? ESTADO_ENTREGA[l.estado_entrega] : undefined
            return (
              <li key={i} className="rounded-lg border bg-white p-4 text-base">
                <p className="font-semibold text-nova-titulo">{resumen(l)}</p>
                <dl className="mt-2 grid gap-1.5 text-base">
                  <div className="flex flex-wrap gap-x-2">
                    <dt className="text-nova-texto-2">Cliente:</dt>
                    <dd className="font-medium text-nova-titulo">{l.cliente ?? '—'}</dd>
                  </div>
                  {l.estado_entrega && (
                    <div className="flex flex-wrap items-center gap-x-2">
                      <dt className="text-nova-texto-2">Estado:</dt>
                      <dd>
                        <EstadoBadge tono={estado?.tono ?? 'neutral'}>{estado?.texto ?? l.estado_entrega}</EstadoBadge>
                      </dd>
                    </div>
                  )}
                  {l.folios.map((f) => {
                    const r = porFolio.get(f)
                    return (
                      <div key={f} className="rounded-md bg-nova-hielo/60 px-2.5 py-2">
                        <dt className="font-semibold text-nova-azul">{f}</dt>
                        <dd className="text-nova-titulo">
                          Ingreso a operaciones pactado con el cliente:{' '}
                          <span className="font-semibold">{r ? formatFecha(r.fecha_inicio_operacion) : '—'}</span>
                        </dd>
                      </div>
                    )
                  })}
                </dl>
                {l.registros_sin_cantidad > 0 && (
                  <p className="mt-1 text-sm text-nova-gris">{l.registros_sin_cantidad} registro(s) sin cantidad informada (no se suman).</p>
                )}
              </li>
            )
          })}
        </ul>
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
  // Fecha pactada de ingreso a operaciones de cada folio (para el detalle del día).
  const reqs = useRequerimientos(false)
  const porFolio = new Map((reqs.data ?? []).filter((r) => r.folio).map((r) => [r.folio!, r]))

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
          <p className="mt-1 text-base text-nova-gris">Entregas de personal por día. Tocá un día para ver el detalle.</p>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="outline" size="icon" onClick={() => mover(-1)} aria-label="Mes anterior">
            <ChevronLeft />
          </Button>
          <p className={cn('min-w-48 text-center text-lg font-semibold text-nova-titulo capitalize', isFetching && 'opacity-60')} aria-live="polite">
            {nombreMes(anio, mes)}
          </p>
          <Button variant="outline" size="icon" onClick={() => mover(1)} aria-label="Mes siguiente">
            <ChevronRight />
          </Button>
        </div>
      </div>

      {error ? (
        <ErrorApi error={error} />
      ) : isPending ? (
        <Skeleton className="h-[520px] rounded-xl" />
      ) : (
        <div className="grid items-start gap-5">
          {/* Calendario (tablet y escritorio) */}
          <div className="hidden overflow-hidden rounded-xl border bg-card shadow-nova-sm md:block">
            <div className="grid grid-cols-7 border-b bg-nova-hielo/50 text-center text-sm font-medium tracking-wide text-nova-gris uppercase">
              {SEMANA.map((s) => (
                <div key={s} className="py-2">
                  {s}
                </div>
              ))}
            </div>
            <div className="grid grid-cols-7">
              {Array.from({ length: vacios }, (_, i) => (
                <div key={`v${i}`} className="min-h-32 border-r border-b bg-muted/30" />
              ))}
              {dias.map((d) => {
                const n = lineas(d).length
                return (
                  <button
                    key={d.fecha}
                    type="button"
                    onClick={() => setSeleccion(d.fecha === seleccion ? null : d.fecha)}
                    aria-pressed={d.fecha === seleccion}
                    aria-label={`${formatFecha(d.fecha)}: ${n === 0 ? 'sin entregas' : lineas(d).map(resumen).join('; ')}`}
                    className={cn(
                      'flex min-h-32 min-w-0 flex-col items-stretch overflow-hidden justify-start border-r border-b p-1.5 text-left transition-colors hover:bg-nova-hielo/40 focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none focus-visible:ring-inset',
                      d.fecha === seleccion && 'bg-nova-hielo',
                    )}
                  >
                    <span
                      className={cn(
                        'inline-grid size-8 place-items-center rounded-full text-base font-medium text-nova-texto-2',
                        d.fecha === hoy && 'bg-nova-azul text-white',
                      )}
                    >
                      {Number(d.fecha.slice(8))}
                    </span>
                    {n > 0 && <Lineas dia={d} max={3} />}
                  </button>
                )
              })}
            </div>
          </div>

          {/* Lista (móvil): solo días con movimientos */}
          <ul className="grid gap-2 md:hidden">
            {dias.filter((d) => lineas(d).length > 0).length === 0 && (
              <li className="rounded-xl border border-dashed bg-card p-8 text-center text-sm text-nova-gris">
                Sin entregas este mes.
              </li>
            )}
            {dias
              .filter((d) => lineas(d).length > 0)
              .map((d) => (
                <li key={d.fecha}>
                  <button
                    type="button"
                    onClick={() => setSeleccion(d.fecha)}
                    className="w-full rounded-xl border bg-card p-3 text-left shadow-nova-xs"
                  >
                    <p className="text-lg font-semibold text-nova-titulo">{formatFecha(d.fecha)}</p>
                    <Lineas dia={d} max={6} />
                  </button>
                </li>
              ))}
          </ul>

          {diaSel && <Detalle dia={diaSel} cerrar={() => setSeleccion(null)} porFolio={porFolio} />}
        </div>
      )}
    </div>
  )
}

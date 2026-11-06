import { AlertTriangle, CheckCircle2, ChevronDown, Layers, type LucideIcon } from 'lucide-react'

import { type Area, useTablero } from '@/api'
import { EstadoBadge, type Tono } from '@/components/EstadoBadge'
import { ErrorApi } from '@/components/ErrorApi'
import { TarjetaSolicitud } from '@/components/solicitudes/TarjetaSolicitud'
import { Skeleton } from '@/components/ui/skeleton'
import { useState } from 'react'

import { cn } from '@/lib/utils'

const COLUMNAS: { key: Area; titulo: string; color: string }[] = [
  { key: 'comercial', titulo: 'Comercial', color: 'bg-nova-naranja' },
  { key: 'operaciones', titulo: 'Operaciones', color: 'bg-nova-azul-medio' },
  { key: 'rrhh', titulo: 'RR.HH.', color: 'bg-nova-azul' },
]

const AREA_TONO: Record<Area, Tono> = { comercial: 'marca', operaciones: 'info', rrhh: 'neutral' }

const TAREAS: Record<string, string> = {
  asignar_contrato_comercial: 'Sin contrato comercial',
  contrato_no_vigente: 'Contrato no vigente',
  verificar_vigencia_contrato: 'Verificar vigencia',
  definir_cargos: 'Sin cargos definidos',
}

const AREA_TITULO: Record<Area, string> = { comercial: 'Comercial', operaciones: 'Operaciones', rrhh: 'RR.HH.' }

type Panel = 'configurados' | 'incompletos' | 'areas'

function Kpi({
  titulo,
  valor,
  icono: Icono,
  tono,
  abierto,
  onClick,
}: {
  titulo: string
  valor: number
  icono: LucideIcon
  tono: 'azul' | 'naranja' | 'rojo'
  abierto: boolean
  onClick: () => void
}) {
  const fondo = { azul: 'bg-nova-hielo text-nova-azul', naranja: 'bg-nova-naranja-claro text-nova-naranja', rojo: 'bg-error/10 text-error' }[tono]
  return (
    <button
      type="button"
      onClick={onClick}
      aria-expanded={abierto}
      className={cn(
        'rounded-xl border bg-card p-5 text-left shadow-nova-sm transition-colors hover:border-nova-azul/50 focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none',
        abierto && 'border-nova-azul ring-1 ring-nova-azul',
      )}
    >
      <div className="flex items-center gap-3">
        <span className={cn('grid size-10 place-items-center rounded-lg', fondo)}>
          <Icono className="size-5" aria-hidden />
        </span>
        <p className="text-sm text-nova-gris">{titulo}</p>
        <ChevronDown className={cn('ml-auto size-4 text-nova-gris transition-transform', abierto && 'rotate-180')} aria-hidden />
      </div>
      <p className="mt-4 text-4xl font-semibold tabular-nums text-nova-titulo">{valor}</p>
    </button>
  )
}

function BarraPorcentaje({ valor }: { valor: number }) {
  return (
    <div className="flex items-center gap-2">
      <div className="h-2 w-24 overflow-hidden rounded-full bg-muted" aria-hidden>
        <div className="h-full rounded-full bg-nova-naranja" style={{ width: `${valor}%` }} />
      </div>
      <span className="text-sm font-medium tabular-nums text-nova-titulo">{valor}%</span>
    </div>
  )
}

export function TableroPage() {
  const { data, isPending, error } = useTablero()
  const [panel, setPanel] = useState<Panel | null>(null)
  const alternar = (p: Panel) => setPanel((actual) => (actual === p ? null : p))

  if (isPending) {
    return (
      <div className="grid gap-4 sm:grid-cols-3">
        {[0, 1, 2].map((i) => (
          <Skeleton key={i} className="h-32 rounded-xl" />
        ))}
      </div>
    )
  }
  if (error) return <ErrorApi error={error} />

  const { indicadores: i, kanban, servicios_configurados, servicios_incompletos, criterios_sin_definir } = data

  // Áreas con al menos una tarea pendiente → cantidad de servicios afectados.
  const porArea = new Map<Area, number>()
  for (const s of servicios_incompletos) {
    for (const area of new Set(s.tareas.map((t) => t.area))) porArea.set(area, (porArea.get(area) ?? 0) + 1)
  }

  return (
    <div className="grid gap-8">
      <section className="grid gap-4 sm:grid-cols-3">
        <Kpi titulo="Servicios configurados" valor={i.servicios_configurados} icono={CheckCircle2} tono="azul" abierto={panel === 'configurados'} onClick={() => alternar('configurados')} />
        <Kpi titulo="Servicios incompletos" valor={i.servicios_incompletos} icono={Layers} tono="naranja" abierto={panel === 'incompletos'} onClick={() => alternar('incompletos')} />
        <Kpi titulo="Áreas sin completar tarea" valor={i.areas_sin_completar} icono={AlertTriangle} tono="rojo" abierto={panel === 'areas'} onClick={() => alternar('areas')} />
      </section>

      {panel === 'configurados' && (
        <section className="rounded-xl border bg-card shadow-nova-sm">
          <h2 className="border-b px-5 py-4 text-lg font-semibold">Servicios configurados</h2>
          {servicios_configurados.length === 0 ? (
            <p className="px-5 py-4 text-sm text-nova-gris">Ningún servicio tiene todos sus datos completos.</p>
          ) : (
            <ul className="divide-y">
              {servicios_configurados.map((s) => (
                <li key={s.servicio.id} className="flex flex-wrap justify-between gap-2 px-5 py-3 text-sm">
                  <span className="font-medium text-nova-titulo">{s.servicio.nombre}</span>
                  <span className="text-nova-texto-2">{s.cliente ?? '—'}</span>
                </li>
              ))}
            </ul>
          )}
        </section>
      )}

      {panel === 'incompletos' && (
        <section className="rounded-xl border bg-card shadow-nova-sm">
          <h2 className="border-b px-5 py-4 text-lg font-semibold">Servicios incompletos</h2>
          {servicios_incompletos.length === 0 ? (
            <p className="px-5 py-4 text-sm text-nova-gris">Todos los servicios están completos.</p>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead className="text-left text-nova-gris">
                  <tr>
                    <th className="px-5 py-3 font-medium">Servicio</th>
                    <th className="px-5 py-3 font-medium">Cliente</th>
                    <th className="px-5 py-3 font-medium">Completitud</th>
                    <th className="px-5 py-3 font-medium">Pendiente</th>
                  </tr>
                </thead>
                <tbody>
                  {servicios_incompletos.map((s) => (
                    <tr key={s.servicio.id} className="border-t">
                      <td className="px-5 py-3 font-medium text-nova-titulo">{s.servicio.nombre}</td>
                      <td className="px-5 py-3 text-nova-texto-2">{s.cliente ?? '—'}</td>
                      <td className="px-5 py-3">
                        <BarraPorcentaje valor={s.porcentaje} />
                      </td>
                      <td className="flex flex-wrap gap-1.5 px-5 py-3">
                        {s.tareas.map((t) => (
                          <EstadoBadge key={t.codigo} tono={AREA_TONO[t.area]}>
                            {TAREAS[t.codigo] ?? t.codigo}
                          </EstadoBadge>
                        ))}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
          {/* A confirmar: el % considera solo los criterios evaluados hoy (contrato vigente y cargos definidos), con igual peso. */}
          <p className="border-t px-5 py-3 text-xs text-nova-gris">
            El % considera los criterios evaluados hoy: contrato comercial vigente y cargos definidos.
            {criterios_sin_definir.length > 0 && <> Aún no se evalúa: {criterios_sin_definir.map((c) => c.replaceAll('_', ' ')).join(', ')}.</>}
          </p>
        </section>
      )}

      {panel === 'areas' && (
        <section className="rounded-xl border bg-card shadow-nova-sm">
          <h2 className="border-b px-5 py-4 text-lg font-semibold">Áreas sin completar tarea</h2>
          {porArea.size === 0 ? (
            <p className="px-5 py-4 text-sm text-nova-gris">Ninguna área tiene tareas pendientes.</p>
          ) : (
            <ul className="divide-y">
              {COLUMNAS.filter((c) => porArea.has(c.key)).map((c) => (
                <li key={c.key} className="flex items-center gap-3 px-5 py-3 text-sm">
                  <span className={cn('size-2.5 rounded-full', c.color)} aria-hidden />
                  <span className="font-medium text-nova-titulo">{AREA_TITULO[c.key]}</span>
                  <span className="ml-auto text-nova-texto-2">{porArea.get(c.key)} servicio(s) con tareas pendientes</span>
                </li>
              ))}
            </ul>
          )}
        </section>
      )}

      <section className="grid gap-4">
        <h2 className="text-lg font-semibold">Cantidad de servicios no completos</h2>
        <div className="grid items-start gap-4 md:grid-cols-3">
          {COLUMNAS.map(({ key, titulo, color }) => (
            <div key={key} className="rounded-xl border bg-muted/40 p-3">
              <div className="mb-3 flex items-center gap-2 px-1">
                <span className={cn('size-2.5 rounded-full', color)} aria-hidden />
                <h3 className="font-semibold">{titulo}</h3>
                <span className="ml-auto rounded-full bg-white px-2 text-xs text-nova-gris shadow-nova-xs">
                  {kanban[key].length}
                </span>
              </div>
              {kanban[key].length === 0 ? (
                <p className="rounded-lg border border-dashed bg-white/60 p-4 text-center text-sm text-nova-gris">
                  Sin solicitudes
                </p>
              ) : (
                <ul className="grid gap-2">
                  {kanban[key].map((t) => (
                    <TarjetaSolicitud key={t.id} t={t} />
                  ))}
                </ul>
              )}
            </div>
          ))}
        </div>
      </section>
    </div>
  )
}

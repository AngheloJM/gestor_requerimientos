import { AlertTriangle, CheckCircle2, Layers, type LucideIcon } from 'lucide-react'

import { type Area, useTablero } from '@/api'
import { EstadoBadge, type Tono } from '@/components/EstadoBadge'
import { ErrorApi } from '@/components/ErrorApi'
import { TarjetaSolicitud } from '@/components/solicitudes/TarjetaSolicitud'
import { Skeleton } from '@/components/ui/skeleton'
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

function Kpi({
  titulo,
  valor,
  de,
  icono: Icono,
  tono,
}: {
  titulo: string
  valor: number
  de: number
  icono: LucideIcon
  tono: 'azul' | 'naranja' | 'rojo'
}) {
  const fondo = { azul: 'bg-nova-hielo text-nova-azul', naranja: 'bg-nova-naranja-claro text-nova-naranja', rojo: 'bg-error/10 text-error' }[tono]
  return (
    <div className="rounded-xl border bg-card p-5 shadow-nova-sm">
      <div className="flex items-center gap-3">
        <span className={cn('grid size-10 place-items-center rounded-lg', fondo)}>
          <Icono className="size-5" aria-hidden />
        </span>
        <p className="text-sm text-nova-gris">{titulo}</p>
      </div>
      <p className="mt-4 text-4xl font-semibold tabular-nums text-nova-titulo">
        {valor}
        <span className="ml-1 text-base font-normal text-nova-gris">/ {de}</span>
      </p>
    </div>
  )
}

export function TableroPage() {
  const { data, isPending, error } = useTablero()

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

  const { indicadores: i, kanban, servicios_incompletos, criterios_sin_definir } = data

  return (
    <div className="grid gap-8">
      <section className="grid gap-4 sm:grid-cols-3">
        <Kpi titulo="Servicios configurados" valor={i.servicios_configurados} de={i.servicios_total} icono={CheckCircle2} tono="azul" />
        <Kpi titulo="Servicios incompletos" valor={i.servicios_incompletos} de={i.servicios_total} icono={Layers} tono="naranja" />
        <Kpi titulo="Áreas sin completar tarea" valor={i.areas_sin_completar} de={3} icono={AlertTriangle} tono="rojo" />
      </section>

      {servicios_incompletos.length > 0 && (
        <section className="rounded-xl border bg-card shadow-nova-sm">
          <h2 className="border-b px-5 py-4 text-lg font-semibold">Servicios con datos pendientes</h2>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead className="text-left text-nova-gris">
                <tr>
                  <th className="px-5 py-3 font-medium">Servicio</th>
                  <th className="px-5 py-3 font-medium">Cliente</th>
                  <th className="px-5 py-3 font-medium">Pendiente</th>
                </tr>
              </thead>
              <tbody>
                {servicios_incompletos.map((s) => (
                  <tr key={s.servicio.id} className="border-t">
                    <td className="px-5 py-3 font-medium text-nova-titulo">{s.servicio.nombre}</td>
                    <td className="px-5 py-3 text-nova-texto-2">{s.cliente ?? '—'}</td>
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
          {criterios_sin_definir.length > 0 && (
            <p className="border-t px-5 py-3 text-xs text-nova-gris">
              Aún no se evalúa: {criterios_sin_definir.join(', ')}.
            </p>
          )}
        </section>
      )}

      <section className="grid gap-4">
        <h2 className="text-lg font-semibold">Solicitudes de cambio</h2>
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

import { Pencil, Plus, SlidersHorizontal } from 'lucide-react'
import { Fragment, useState } from 'react'

import { useMatrizServicio } from '@/api'
import { ErrorApi } from '@/components/ErrorApi'
import { EditarHaberes, NuevaComposicion } from '@/components/matriz/ComposicionForms'
import { MetricasEditor } from '@/components/matriz/MetricasEditor'
import { Button } from '@/components/ui/button'
import { Skeleton } from '@/components/ui/skeleton'
import { formatMonto } from '@/lib/format'

const bs = (m: string | null) => (m == null ? '—' : `Bs ${formatMonto(m)}`)

/**
 * Cargos de un servicio, dentro de su formulario (revisión 04/11/2026): cada cargo trae su
 * composición salarial (haberes), cantidades y métricas con bonos. Los cargos compartidos entre
 * varios centros de costo se distribuyen en la pestaña "Composición salarial".
 */
export function CargosServicio({ servicioId, esRrhh }: { servicioId: string; esRrhh: boolean }) {
  const { data, isPending, error } = useMatrizServicio()
  const filas = (data ?? []).filter((f) => f.servicio.id === servicioId)
  const [agregando, setAgregando] = useState(false)
  const [abierto, setAbierto] = useState<{ id: string; modo: 'editar' | 'metricas' } | null>(null)
  const alternar = (id: string, modo: 'editar' | 'metricas') =>
    setAbierto(abierto?.id === id && abierto.modo === modo ? null : { id, modo })

  return (
    <section className="grid gap-3 rounded-xl border bg-nova-hielo/30 p-4" aria-label="Cargos del servicio">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div>
          <h4 className="font-semibold text-nova-titulo">Cargos de este servicio</h4>
          <p className="text-xs text-nova-gris">Cargo, composición salarial, cantidades, tiempos de capacitación y métricas.</p>
        </div>
        {esRrhh && (
          <Button size="sm" onClick={() => setAgregando(!agregando)} aria-expanded={agregando}>
            <Plus aria-hidden /> Agregar cargo
          </Button>
        )}
      </div>

      {agregando && (
        <NuevaComposicion
          servicioId={servicioId}
          excluirCargos={filas.map((f) => f.cargo.id)}
          cerrar={() => setAgregando(false)}
          onCreada={(id) => setAbierto({ id, modo: 'metricas' })}
        />
      )}

      {error ? (
        <ErrorApi error={error} />
      ) : isPending ? (
        <Skeleton className="h-24 rounded-lg" />
      ) : filas.length === 0 ? (
        <p className="rounded-lg border border-dashed bg-white p-4 text-center text-sm text-nova-gris">
          Este servicio todavía no tiene cargos.
        </p>
      ) : (
        <div className="overflow-x-auto rounded-lg border bg-white">
          <table className="w-full min-w-[900px] text-sm">
            <thead className="bg-nova-hielo text-left text-xs tracking-wide text-nova-gris uppercase">
              <tr>
                <th className="px-3 py-2 font-medium">Cargo</th>
                <th className="px-3 py-2 text-right font-medium">Haber diurno</th>
                <th className="px-3 py-2 text-right font-medium">Haber nocturno</th>
                <th className="px-3 py-2 text-right font-medium">Mujeres</th>
                <th className="px-3 py-2 text-right font-medium">Hombres</th>
                <th className="px-3 py-2 text-right font-medium whitespace-nowrap">Capacitación / OJT / ajustada</th>
                <th className="px-3 py-2 text-right font-medium">Total ganado</th>
                <th className="px-3 py-2" aria-label="Acciones" />
              </tr>
            </thead>
            <tbody>
              {filas.map((f) => (
                <Fragment key={f.servicio_cargo_id}>
                  <tr className="border-t align-top">
                    <td className="px-3 py-2 font-medium text-nova-titulo">{f.cargo.nombre}</td>
                    <td className="px-3 py-2 text-right tabular-nums">{bs(f.haber_basico_diurno)}</td>
                    <td className="px-3 py-2 text-right tabular-nums">{bs(f.haber_basico_nocturno)}</td>
                    <td className="px-3 py-2 text-right tabular-nums">{f.cantidad_mujeres ?? '—'}</td>
                    <td className="px-3 py-2 text-right tabular-nums">{f.cantidad_hombres ?? '—'}</td>
                    <td className="px-3 py-2 text-right whitespace-nowrap tabular-nums">
                      {[f.tiempo_capacitacion_dias, f.tiempo_ojt_dias, f.tiempo_capacitacion_ajustada_dias].map((d) => (d != null ? `${d} d` : '—')).join(' · ')}
                    </td>
                    <td className="px-3 py-2 text-right font-semibold tabular-nums text-nova-azul">{bs(f.total_ganado)}</td>
                    <td className="px-3 py-1.5 text-right whitespace-nowrap">
                      {esRrhh && (
                        <>
                          <Button
                            variant="ghost"
                            size="sm"
                            aria-expanded={abierto?.id === f.servicio_cargo_id && abierto.modo === 'editar'}
                            onClick={() => alternar(f.servicio_cargo_id, 'editar')}
                          >
                            <Pencil aria-hidden /> Editar
                          </Button>
                          <Button
                            variant="ghost"
                            size="sm"
                            aria-expanded={abierto?.id === f.servicio_cargo_id && abierto.modo === 'metricas'}
                            onClick={() => alternar(f.servicio_cargo_id, 'metricas')}
                          >
                            <SlidersHorizontal aria-hidden /> Métricas ({f.metricas.length})
                          </Button>
                        </>
                      )}
                    </td>
                  </tr>
                  {abierto?.id === f.servicio_cargo_id && (
                    <tr>
                      <td colSpan={8} className="border-t bg-nova-hielo/30 px-3 py-3">
                        <p className="mb-2 text-xs text-nova-gris">
                          {abierto.modo === 'editar' ? 'Composición salarial, cantidades y tiempos' : 'Métricas y bonos'} · {f.cargo.nombre}
                        </p>
                        {abierto.modo === 'editar' ? (
                          <EditarHaberes fila={f} cerrar={() => setAbierto(null)} />
                        ) : (
                          <MetricasEditor servicioCargoId={f.servicio_cargo_id} esRrhh={esRrhh} />
                        )}
                      </td>
                    </tr>
                  )}
                </Fragment>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  )
}

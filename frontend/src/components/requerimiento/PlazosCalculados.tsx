import { usePlazos } from '@/api'
import { ErrorApi } from '@/components/ErrorApi'
import { formatFecha } from '@/lib/format'

function Dato({ etiqueta, valor, nota }: { etiqueta: string; valor: string; nota?: string }) {
  return (
    <div className="rounded-lg border bg-nova-hielo/40 p-3">
      <p className="text-xs text-nova-gris">{etiqueta}</p>
      <p className="mt-1 text-base font-semibold tabular-nums text-nova-titulo">{valor}</p>
      {nota && <p className="mt-0.5 text-xs text-nova-gris">{nota}</p>}
    </div>
  )
}

/**
 * Revisión 06/11/2026 (puntos 10 y 11): al elegir la fecha de inicio de operación se calculan
 * las fechas máximas de capacitación (normal y ajustada) y los días hábiles de reclutamiento.
 * Solo se muestran: no se guardan en el requerimiento.
 */
export function PlazosCalculados({ operacion, servicioId, cargoId }: { operacion: string; servicioId?: string; cargoId?: string }) {
  const { data: p, error, isFetching } = usePlazos(operacion, servicioId, cargoId)
  if (!operacion) return <p className="text-sm text-nova-gris">Elegí la fecha de inicio de operación para ver los plazos calculados.</p>
  if (error) return <ErrorApi error={error} />
  if (!p) return <p className="text-sm text-nova-gris">{isFetching ? 'Calculando plazos…' : ''}</p>

  const sinDato = 'Sin dato'
  const entregaEsAjustada = !!p.capacitacion_ajustada
  return (
    <div className="grid gap-3 sm:grid-cols-3" aria-live="polite">
      <Dato
        etiqueta="Fecha de inicio máximo de capacitación"
        valor={p.capacitacion_normal ? formatFecha(p.capacitacion_normal) : sinDato}
        nota={
          p.origen_normal === 'servicio'
            ? 'Con el tiempo general del servicio (el cargo no tiene uno propio).'
            : p.capacitacion_normal
              ? undefined
              : 'El cargo y el servicio no tienen tiempo de capacitación cargado.'
        }
      />
      <Dato
        etiqueta="Fecha de inicio máximo de capacitación ajustada"
        valor={p.capacitacion_ajustada ? formatFecha(p.capacitacion_ajustada) : sinDato}
        nota={p.capacitacion_ajustada ? undefined : 'El cargo no tiene tiempo de capacitación ajustada cargado.'}
      />
      <Dato
        etiqueta="Tiempo de reclutamiento"
        valor={p.dias_reclutamiento == null ? sinDato : `${p.dias_reclutamiento} ${p.dias_reclutamiento === 1 ? 'día hábil' : 'días hábiles'}`}
        nota={
          p.entrega_capacitacion
            ? `Desde hoy (${formatFecha(p.hoy)}) hasta la entrega a capacitación${entregaEsAjustada ? ' ajustada' : ''} (${formatFecha(p.entrega_capacitacion)}), sin fines de semana ni feriados.`
            : undefined
        }
      />
    </div>
  )
}

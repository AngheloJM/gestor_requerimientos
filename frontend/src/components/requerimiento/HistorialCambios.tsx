import { useHistorialAprobacion } from '@/api'
import { ErrorApi } from '@/components/ErrorApi'
import { Skeleton } from '@/components/ui/skeleton'
import { formatFecha } from '@/lib/format'

export const CATEGORIAS_DESVIO: [string, string][] = [
  ['falla_capacitacion', 'Falla de capacitación'],
  ['rechazo_operaciones', 'Rechazo de operaciones'],
  ['ratio_reclutamiento', 'Ratio de reclutamiento'],
  ['cambio_cliente', 'Cambio del cliente'],
]
const DESVIO = Object.fromEntries(CATEGORIAS_DESVIO)

const CAMPO: Record<string, string> = {
  cantidad_personas: 'Cantidad requerida',
  fecha_fin: 'Fecha de fin',
  estado: 'Estado',
  fecha_inicio_operacion: 'Inicio de operación',
  fecha_inicio_capacitacion: 'Inicio de capacitación',
  motivo: 'Motivo',
  comentarios_adicionales: 'Comentarios',
}

const texto = (v: string | null | undefined) => (v == null ? '—' : /^\d{4}-\d{2}-\d{2}$/.test(v) ? formatFecha(v) : v)

/** Historial de cambios (requerimiento_historial), del más reciente al más antiguo. */
export function HistorialCambios({ id }: { id: string }) {
  const { data, isPending, error } = useHistorialAprobacion(id)
  if (error) return <ErrorApi error={error} />
  if (isPending) return <Skeleton className="h-16" />
  if (!data.length) return <p className="text-sm text-nova-gris">Sin cambios registrados.</p>
  return (
    <ol className="grid gap-2">
      {[...data].reverse().map((h) => (
        <li key={h.id} className="rounded-lg border bg-white p-2.5 text-sm">
          <p className="text-nova-titulo">
            <span className="font-medium">{CAMPO[h.campo] ?? h.campo}</span>: {texto(h.valor_anterior)} → {texto(h.valor_nuevo)}
          </p>
          {h.glosa && <p className="mt-0.5 text-nova-texto-2">“{h.glosa}”</p>}
          {h.categoria_desvio && (
            <p className="mt-0.5 text-xs text-nova-texto-2">
              Desvío: {DESVIO[h.categoria_desvio] ?? h.categoria_desvio}
              {h.detalle_desvio ? ` · ${h.detalle_desvio}` : ''}
            </p>
          )}
          <p className="mt-0.5 text-xs text-nova-gris">
            {formatFecha(h.created_at.slice(0, 10))}
            {h.usuario_nombre ? ` · ${h.usuario_nombre}` : ''}
          </p>
        </li>
      ))}
    </ol>
  )
}

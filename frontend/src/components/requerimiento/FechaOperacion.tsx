import { Pencil } from 'lucide-react'
import { useState } from 'react'

import { type RequerimientoDetalle, useEditarConTrazabilidad } from '@/api'
import { EdicionTrazable, entradaTrazable } from '@/components/requerimiento/EdicionTrazable'
import { Button } from '@/components/ui/button'
import { formatFecha } from '@/lib/format'

const FECHA = /^\d{4}-\d{2}-\d{2}$/

/**
 * Fecha de inicio de operación editable con el mismo mecanismo que la cantidad (revisión
 * 06/11/2026, punto 15): glosa obligatoria, desvío opcional y registro en el historial por
 * usuario. No mueve la fecha de capacitación.
 */
export function FechaOperacion({ r }: { r: RequerimientoDetalle }) {
  const editar = useEditarConTrazabilidad()
  const actual = r.fecha_inicio_operacion ?? ''
  const [abierto, setAbierto] = useState(false)
  const [nueva, setNueva] = useState(actual)

  const abrir = () => {
    setNueva(actual)
    editar.reset()
    setAbierto(true)
  }
  const validar = () =>
    !FECHA.test(nueva) ? 'Indicá la nueva fecha de inicio de operación.' : nueva === actual ? 'La fecha es la misma que la actual.' : undefined

  return (
    <div className="grid gap-4">
      <div className="flex flex-wrap items-center justify-between gap-3 rounded-lg border p-3">
        <div>
          <p className="text-xs text-nova-gris">Fecha de inicio de operación</p>
          <p className="text-lg font-semibold tabular-nums text-nova-titulo">{actual ? formatFecha(actual) : '—'}</p>
        </div>
        {!abierto && (
          <Button variant="outline" size="sm" onClick={abrir}>
            <Pencil aria-hidden /> Cambiar fecha
          </Button>
        )}
      </div>

      {abierto && (
        <EdicionTrazable
          etiqueta="Cambiar fecha de inicio de operación"
          textoGuardar="Guardar fecha"
          validar={validar}
          pendiente={editar.isPending}
          error={editar.error}
          cancelar={() => setAbierto(false)}
          guardar={(t) => editar.mutate({ id: r.id, fecha_inicio_operacion: nueva, version: r.version, ...t }, { onSuccess: () => setAbierto(false) })}
          campo={
            <label className="grid gap-1 text-xs text-nova-gris">
              Nueva fecha de inicio de operación
              <input type="date" className={`${entradaTrazable} w-48`} value={nueva} onChange={(e) => setNueva(e.target.value)} />
            </label>
          }
        />
      )}
    </div>
  )
}

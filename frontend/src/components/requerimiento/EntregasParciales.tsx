import { Minus, Pencil, Plus } from 'lucide-react'
import { useState } from 'react'

import { type RequerimientoDetalle, useEditarConTrazabilidad } from '@/api'
import { EdicionTrazable, entradaTrazable } from '@/components/requerimiento/EdicionTrazable'
import { Button } from '@/components/ui/button'

function Indicador({ titulo, valor, de }: { titulo: string; valor: number | string; de: number | string }) {
  return (
    <div className="rounded-lg border bg-nova-hielo/40 p-3">
      <p className="text-xs text-nova-gris">{titulo}</p>
      <p className="mt-1 text-xl font-semibold tabular-nums text-nova-titulo">
        {valor}
        <span className="text-base font-normal text-nova-gris"> / {de}</span>
      </p>
    </div>
  )
}

/**
 * Cantidad actual vs. lo aceptado/informado, y edición de la **cantidad actual** por la función
 * oficial (`editar_requerimiento_con_historial`): glosa obligatoria, desvío opcional (categoría +
 * explicación juntas), registro en el historial. La cantidad original no se modifica.
 */
export function EntregasParciales({ r }: { r: RequerimientoDetalle }) {
  const editar = useEditarConTrazabilidad()
  const actual = r.cantidad ?? 0
  const [abierto, setAbierto] = useState(false)
  const [nueva, setNueva] = useState(String(actual))

  const abrir = () => {
    setNueva(String(actual))
    editar.reset()
    setAbierto(true)
  }
  const n = Number(nueva)
  const paso = (d: number) => setNueva(String(Math.max(1, (Number.isInteger(n) ? n : actual) + d)))

  const validar = () =>
    !/^[1-9]\d*$/.test(nueva.trim())
      ? 'La cantidad debe ser un número entero mayor a cero.'
      : n === actual
        ? 'La cantidad es la misma que la actual.'
        : n < r.total_aceptado
          ? `No puede ser menor a lo ya aceptado (${r.total_aceptado}).`
          : undefined

  return (
    <div className="grid gap-4">
      <div className="grid gap-3 sm:grid-cols-2">
        <Indicador titulo="Total aceptado / cantidad actual" valor={r.total_aceptado} de={actual} />
        <Indicador titulo="Reclutados informados / cantidad original" valor={r.reclutados_informados} de={r.cantidad_original ?? '—'} />
      </div>

      <div className="flex flex-wrap items-center justify-between gap-3 rounded-lg border p-3">
        <div>
          <p className="text-xs text-nova-gris">Cantidad requerida (actual)</p>
          <p className="text-lg font-semibold tabular-nums text-nova-titulo">{actual}</p>
          {r.cantidad_original != null && <p className="text-xs text-nova-gris">Original: {r.cantidad_original} (no se modifica)</p>}
        </div>
        {!abierto && (
          <Button variant="outline" size="sm" onClick={abrir}>
            <Pencil aria-hidden /> Cambiar cantidad
          </Button>
        )}
      </div>

      {abierto && (
        <EdicionTrazable
          etiqueta="Cambiar cantidad requerida"
          textoGuardar="Guardar cantidad"
          validar={validar}
          pendiente={editar.isPending}
          error={editar.error}
          cancelar={() => setAbierto(false)}
          guardar={(t) => editar.mutate({ id: r.id, cantidad: n, version: r.version, ...t }, { onSuccess: () => setAbierto(false) })}
          campo={
            <label className="grid gap-1 text-xs text-nova-gris">
              Nueva cantidad requerida
              <span className="flex items-center gap-2">
                <Button type="button" variant="outline" size="icon" onClick={() => paso(-1)} aria-label="Restar uno">
                  <Minus />
                </Button>
                <input className={`${entradaTrazable} w-24 text-center tabular-nums`} inputMode="numeric" value={nueva} onChange={(e) => setNueva(e.target.value)} />
                <Button type="button" variant="outline" size="icon" onClick={() => paso(1)} aria-label="Sumar uno">
                  <Plus />
                </Button>
              </span>
            </label>
          }
        />
      )}
    </div>
  )
}

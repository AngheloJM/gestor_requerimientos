import { Minus, Pencil, Plus } from 'lucide-react'
import { useState } from 'react'

import { type CategoriaDesvio, type RequerimientoDetalle, useEditarCantidad } from '@/api'
import { ErrorApi } from '@/components/ErrorApi'
import { CATEGORIAS_DESVIO } from '@/components/requerimiento/HistorialCambios'
import { Button } from '@/components/ui/button'

const entrada =
  'h-9 rounded-lg border border-input bg-white px-2.5 text-sm outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50'

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
  const editar = useEditarCantidad()
  const actual = r.cantidad ?? 0
  const [abierto, setAbierto] = useState(false)
  const [nueva, setNueva] = useState(String(actual))
  const [glosa, setGlosa] = useState('')
  const [categoria, setCategoria] = useState('')
  const [detalle, setDetalle] = useState('')
  const [aviso, setAviso] = useState<string>()

  const abrir = () => {
    setNueva(String(actual))
    setGlosa('')
    setCategoria('')
    setDetalle('')
    setAviso(undefined)
    editar.reset()
    setAbierto(true)
  }
  const n = Number(nueva)
  const paso = (d: number) => setNueva(String(Math.max(1, (Number.isInteger(n) ? n : actual) + d)))

  const guardar = (e: React.FormEvent) => {
    e.preventDefault()
    const msg = !/^[1-9]\d*$/.test(nueva.trim())
      ? 'La cantidad debe ser un número entero mayor a cero.'
      : n === actual
        ? 'La cantidad es la misma que la actual.'
        : n < r.total_aceptado
          ? `No puede ser menor a lo ya aceptado (${r.total_aceptado}).`
          : glosa.trim().length < 3
            ? 'Escribí la glosa (al menos 3 caracteres): queda en el historial.'
            : categoria && detalle.trim().length < 3
              ? 'Explicá el desvío (al menos 3 caracteres).'
              : undefined
    setAviso(msg)
    if (msg) return
    editar.mutate(
      {
        id: r.id,
        cantidad: n,
        version: r.version,
        glosa: glosa.trim(),
        ...(categoria ? { categoria_desvio: categoria as CategoriaDesvio, detalle_desvio: detalle.trim() } : {}),
      },
      { onSuccess: () => setAbierto(false) },
    )
  }

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
        <form onSubmit={guardar} className="grid gap-3 rounded-lg border border-nova-azul/30 bg-nova-hielo/30 p-4" aria-label="Cambiar cantidad requerida">
          <p className="text-sm font-semibold text-nova-titulo">Trazabilidad de la edición</p>
          <label className="grid gap-1 text-xs text-nova-gris">
            Nueva cantidad requerida
            <span className="flex items-center gap-2">
              <Button type="button" variant="outline" size="icon" onClick={() => paso(-1)} aria-label="Restar uno">
                <Minus />
              </Button>
              <input className={`${entrada} w-24 text-center tabular-nums`} inputMode="numeric" value={nueva} onChange={(e) => setNueva(e.target.value)} />
              <Button type="button" variant="outline" size="icon" onClick={() => paso(1)} aria-label="Sumar uno">
                <Plus />
              </Button>
            </span>
          </label>
          <label className="grid gap-1 text-xs text-nova-gris">
            Glosa (obligatoria)
            <textarea className={`${entrada} h-auto py-2`} rows={2} maxLength={2000} value={glosa} onChange={(e) => setGlosa(e.target.value)} />
          </label>
          <div className="grid gap-3 sm:grid-cols-2">
            <label className="grid gap-1 text-xs text-nova-gris">
              Motivo del desvío (si corresponde)
              <select className={entrada} value={categoria} onChange={(e) => setCategoria(e.target.value)}>
                <option value="">Sin desvío</option>
                {CATEGORIAS_DESVIO.map(([v, t]) => (
                  <option key={v} value={v}>
                    {t}
                  </option>
                ))}
              </select>
            </label>
            {categoria && (
              <label className="grid gap-1 text-xs text-nova-gris">
                Explicación del desvío (obligatoria)
                <input className={entrada} maxLength={2000} value={detalle} onChange={(e) => setDetalle(e.target.value)} />
              </label>
            )}
          </div>
          {aviso && (
            <p className="text-sm text-error" role="alert">
              {aviso}
            </p>
          )}
          {editar.error ? <ErrorApi error={editar.error} /> : null}
          <div className="flex justify-end gap-2">
            <Button type="button" variant="outline" onClick={() => setAbierto(false)} disabled={editar.isPending}>
              Cancelar
            </Button>
            <Button type="submit" disabled={editar.isPending}>
              {editar.isPending ? 'Guardando…' : 'Guardar cantidad'}
            </Button>
          </div>
        </form>
      )}
    </div>
  )
}

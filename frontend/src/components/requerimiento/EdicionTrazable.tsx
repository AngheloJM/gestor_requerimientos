import { type ReactNode, useState } from 'react'

import type { CategoriaDesvio } from '@/api'
import { ErrorApi } from '@/components/ErrorApi'
import { CATEGORIAS_DESVIO } from '@/components/requerimiento/HistorialCambios'
import { Button } from '@/components/ui/button'

export const entradaTrazable =
  'h-9 rounded-lg border border-input bg-white px-2.5 text-sm outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50'

export interface Trazabilidad {
  glosa: string
  categoria_desvio?: CategoriaDesvio
  detalle_desvio?: string
}

/**
 * Formulario "Trazabilidad de la edición", común a los campos que se editan por la función
 * oficial `editar_requerimiento_con_historial` (cantidad requerida, fecha de inicio de
 * operación): glosa obligatoria, desvío opcional (categoría + explicación juntas). El valor
 * nuevo lo aporta `campo`; `validar` revisa ese valor antes de la glosa.
 */
export function EdicionTrazable({
  etiqueta,
  campo,
  validar,
  guardar,
  cancelar,
  pendiente,
  error,
  textoGuardar,
}: {
  etiqueta: string
  campo: ReactNode
  validar: () => string | undefined
  guardar: (t: Trazabilidad) => void
  cancelar: () => void
  pendiente: boolean
  error: unknown
  textoGuardar: string
}) {
  const [glosa, setGlosa] = useState('')
  const [categoria, setCategoria] = useState('')
  const [detalle, setDetalle] = useState('')
  const [aviso, setAviso] = useState<string>()

  const enviar = (e: React.FormEvent) => {
    e.preventDefault()
    const msg =
      validar() ??
      (glosa.trim().length < 3
        ? 'Escribí la glosa (al menos 3 caracteres): queda en el historial.'
        : categoria && detalle.trim().length < 3
          ? 'Explicá el desvío (al menos 3 caracteres).'
          : undefined)
    setAviso(msg)
    if (msg) return
    guardar({
      glosa: glosa.trim(),
      ...(categoria ? { categoria_desvio: categoria as CategoriaDesvio, detalle_desvio: detalle.trim() } : {}),
    })
  }

  return (
    <form onSubmit={enviar} className="grid gap-3 rounded-lg border border-nova-azul/30 bg-nova-hielo/30 p-4" aria-label={etiqueta}>
      <p className="text-sm font-semibold text-nova-titulo">Trazabilidad de la edición</p>
      {campo}
      <label className="grid gap-1 text-xs text-nova-gris">
        Glosa (obligatoria)
        <textarea className={`${entradaTrazable} h-auto py-2`} rows={2} maxLength={2000} value={glosa} onChange={(e) => setGlosa(e.target.value)} />
      </label>
      <div className="grid gap-3 sm:grid-cols-2">
        <label className="grid gap-1 text-xs text-nova-gris">
          Motivo del desvío (si corresponde)
          <select className={entradaTrazable} value={categoria} onChange={(e) => setCategoria(e.target.value)}>
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
            <input className={entradaTrazable} maxLength={2000} value={detalle} onChange={(e) => setDetalle(e.target.value)} />
          </label>
        )}
      </div>
      {aviso && (
        <p className="text-sm text-error" role="alert">
          {aviso}
        </p>
      )}
      {error ? <ErrorApi error={error} /> : null}
      <div className="flex justify-end gap-2">
        <Button type="button" variant="outline" onClick={cancelar} disabled={pendiente}>
          Cancelar
        </Button>
        <Button type="submit" disabled={pendiente}>
          {pendiente ? 'Guardando…' : textoGuardar}
        </Button>
      </div>
    </form>
  )
}

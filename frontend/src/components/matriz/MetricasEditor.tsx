import { Plus, Trash2 } from 'lucide-react'
import { useState } from 'react'

import {
  type MetricaAsignada,
  type MetricaCatalogoCfg,
  type UmbralCfg,
  useAsignarMetrica,
  useConfigGuardar,
  useConfigLista,
  useEditarMetricaAsignada,
  useMetricasAsignadas,
  useQuitarMetricaAsignada,
} from '@/api'
import { ErrorApi } from '@/components/ErrorApi'
import { Button } from '@/components/ui/button'
import { Skeleton } from '@/components/ui/skeleton'
import { formatMonto } from '@/lib/format'

const DECIMAL = /^\d{1,9}(\.\d{1,4})?$/
/** Valor del selector para crear una métrica nueva en el catálogo. */
const NUEVA = '__nueva__'
const celda = 'h-8 w-full rounded-md border border-input bg-white px-2 text-sm tabular-nums disabled:bg-muted'

type Tramo = { intervalo_min: string; intervalo_max: string; pct_bono: string; observaciones: string }

const aTramo = (u: UmbralCfg): Tramo => ({
  intervalo_min: u.intervalo_min ?? '',
  intervalo_max: u.intervalo_max ?? '',
  pct_bono: u.pct_bono,
  observaciones: u.observaciones ?? '',
})

/** Una métrica asignada: peso, monto y su tabla de escalas de cumplimiento. */
function Metrica({ m, esRrhh }: { m: MetricaAsignada; esRrhh: boolean }) {
  const editar = useEditarMetricaAsignada()
  const quitar = useQuitarMetricaAsignada()
  const [peso, setPeso] = useState(m.peso_pct)
  const [monto, setMonto] = useState(m.monto_bono)
  const [tramos, setTramos] = useState<Tramo[]>(m.umbrales.map(aTramo))
  const [aviso, setAviso] = useState<string>()
  const [ok, setOk] = useState(false)
  const [confirmar, setConfirmar] = useState(false)

  const cambiarTramo = (i: number, k: keyof Tramo, v: string) => {
    setTramos(tramos.map((t, j) => (j === i ? { ...t, [k]: v } : t)))
    setOk(false)
  }

  const guardar = () => {
    if (!DECIMAL.test(peso) || !DECIMAL.test(monto)) return setAviso('Peso y monto: números no negativos (punto decimal).')
    for (const [i, t] of tramos.entries()) {
      for (const v of [t.intervalo_min, t.intervalo_max]) if (v && !DECIMAL.test(v)) return setAviso(`Tramo ${i + 1}: los límites deben ser números.`)
      if (!DECIMAL.test(t.pct_bono)) return setAviso(`Tramo ${i + 1}: indicá el % del bono.`)
      if (t.intervalo_min && t.intervalo_max && Number(t.intervalo_min) > Number(t.intervalo_max))
        return setAviso(`Tramo ${i + 1}: el mínimo supera al máximo.`)
    }
    setAviso(undefined)
    editar.mutate(
      {
        id: m.id,
        body: {
          peso_pct: peso,
          monto_bono: monto,
          umbrales: tramos.map((t, i) => ({
            intervalo_min: t.intervalo_min || null,
            intervalo_max: t.intervalo_max || null,
            pct_bono: t.pct_bono,
            observaciones: t.observaciones.trim() || null,
            orden: i + 1,
          })),
        },
      },
      { onSuccess: () => setOk(true) },
    )
  }

  return (
    <li className="grid gap-3 rounded-lg border bg-white p-3">
      <div className="flex flex-wrap items-end gap-3">
        <p className="mr-auto font-medium text-nova-titulo">{m.nombre}</p>
        <label className="grid gap-1 text-xs text-nova-gris">
          Peso (%)
          <input className={`${celda} w-24`} disabled={!esRrhh} value={peso} onChange={(e) => (setPeso(e.target.value), setOk(false))} />
        </label>
        <label className="grid gap-1 text-xs text-nova-gris">
          Monto del bono (Bs)
          <input className={`${celda} w-32`} disabled={!esRrhh} value={monto} onChange={(e) => (setMonto(e.target.value), setOk(false))} />
        </label>
      </div>

      <div className="overflow-x-auto">
        <p className="mb-1 text-xs font-medium text-nova-texto-2">Escalas de cumplimiento</p>
        <table className="w-full min-w-[560px] text-sm">
          <thead className="text-left text-xs text-nova-gris">
            <tr>
              <th className="py-1 pr-2 font-medium">Desde (%)</th>
              <th className="py-1 pr-2 font-medium">Hasta (%)</th>
              <th className="py-1 pr-2 font-medium">% del bono</th>
              <th className="py-1 pr-2 font-medium">Observaciones</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {tramos.length === 0 && (
              <tr>
                <td colSpan={5} className="py-2 text-xs text-nova-gris">
                  Sin escalas.
                </td>
              </tr>
            )}
            {tramos.map((t, i) => (
              <tr key={i}>
                <td className="py-1 pr-2">
                  <input aria-label={`Tramo ${i + 1} desde`} className={celda} disabled={!esRrhh} value={t.intervalo_min} onChange={(e) => cambiarTramo(i, 'intervalo_min', e.target.value)} />
                </td>
                <td className="py-1 pr-2">
                  <input aria-label={`Tramo ${i + 1} hasta`} className={celda} disabled={!esRrhh} value={t.intervalo_max} onChange={(e) => cambiarTramo(i, 'intervalo_max', e.target.value)} />
                </td>
                <td className="py-1 pr-2">
                  <input aria-label={`Tramo ${i + 1} % del bono`} className={celda} disabled={!esRrhh} value={t.pct_bono} onChange={(e) => cambiarTramo(i, 'pct_bono', e.target.value)} />
                </td>
                <td className="py-1 pr-2">
                  <input aria-label={`Tramo ${i + 1} observaciones`} className={celda} disabled={!esRrhh} value={t.observaciones} onChange={(e) => cambiarTramo(i, 'observaciones', e.target.value)} />
                </td>
                <td className="py-1 text-right">
                  {esRrhh && (
                    <Button variant="ghost" size="icon" aria-label={`Quitar tramo ${i + 1}`} onClick={() => (setTramos(tramos.filter((_, j) => j !== i)), setOk(false))}>
                      <Trash2 />
                    </Button>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        <p className="mt-1 text-xs text-nova-gris">Escala del % del bono (0–100 o 0–1) pendiente de confirmar: se guarda tal cual se escribe.</p>
      </div>

      {aviso && (
        <p className="text-xs text-error" role="alert">
          {aviso}
        </p>
      )}
      {editar.error ? <ErrorApi error={editar.error} /> : null}
      {quitar.error ? <ErrorApi error={quitar.error} /> : null}
      {esRrhh && (
        <div className="flex flex-wrap items-center gap-2">
          <Button size="sm" variant="outline" onClick={() => (setTramos([...tramos, { intervalo_min: '', intervalo_max: '', pct_bono: '', observaciones: '' }]), setOk(false))}>
            <Plus aria-hidden /> Agregar tramo
          </Button>
          <span className="mr-auto" />
          {ok && (
            <span className="text-xs text-exito" role="status">
              Guardado
            </span>
          )}
          {confirmar ? (
            <span className="flex items-center gap-1.5 text-xs">
              ¿Quitar la métrica y sus escalas?
              <Button size="sm" variant="destructive" disabled={quitar.isPending} onClick={() => quitar.mutate(m.id)}>
                Sí
              </Button>
              <Button size="sm" variant="ghost" onClick={() => setConfirmar(false)}>
                No
              </Button>
            </span>
          ) : (
            <Button size="sm" variant="ghost" className="text-error" onClick={() => setConfirmar(true)}>
              Quitar métrica
            </Button>
          )}
          <Button size="sm" disabled={editar.isPending} onClick={guardar}>
            Guardar métrica
          </Button>
        </div>
      )}
    </li>
  )
}

/** Métricas y bonos de un servicio_cargo (la métrica se elige del catálogo de métricas). */
export function MetricasEditor({ servicioCargoId, esRrhh }: { servicioCargoId: string; esRrhh: boolean }) {
  const asignadas = useMetricasAsignadas(servicioCargoId)
  const catalogo = useConfigLista<MetricaCatalogoCfg>('catalogo-metricas')
  const asignar = useAsignarMetrica(servicioCargoId)
  // Alta de una métrica en el catálogo sin salir de la fila (no hay pestaña de catálogo).
  const crearEnCatalogo = useConfigGuardar('catalogo-metricas')
  const [nombreNueva, setNombreNueva] = useState('')
  const [metrica, setMetrica] = useState('')
  const [peso, setPeso] = useState('')
  const [monto, setMonto] = useState('')
  const [aviso, setAviso] = useState<string>()

  if (asignadas.error) return <ErrorApi error={asignadas.error} />
  if (asignadas.isPending) return <Skeleton className="h-24 rounded-lg" />

  const usadas = new Set(asignadas.data.map((m) => m.metrica_id))
  const disponibles = (catalogo.data ?? []).filter((m) => !usadas.has(m.id))
  const total = asignadas.data.reduce((s, m) => s + Number(m.monto_bono), 0)

  const crearMetrica = () => {
    if (!nombreNueva.trim()) return setAviso('Escribí el nombre de la nueva métrica.')
    setAviso(undefined)
    crearEnCatalogo.mutate(
      { body: { nombre: nombreNueva.trim(), concepto: null, formula: null } },
      { onSuccess: ({ id }) => (setMetrica(id), setNombreNueva('')) },
    )
  }

  const agregar = () => {
    if (!metrica || metrica === NUEVA) return setAviso('Elegí una métrica del catálogo.')
    if (!DECIMAL.test(peso) || !DECIMAL.test(monto)) return setAviso('Peso y monto: números no negativos (punto decimal).')
    setAviso(undefined)
    asignar.mutate({ metrica_id: metrica, peso_pct: peso, monto_bono: monto }, { onSuccess: () => (setMetrica(''), setPeso(''), setMonto('')) })
  }

  return (
    <div className="grid gap-3">
      <p className="text-xs text-nova-gris">
        {asignadas.data.length} métrica(s) · bonos configurados: Bs {formatMonto(total.toFixed(2))}
      </p>
      <ul className="grid gap-2">
        {asignadas.data.map((m) => (
          <Metrica key={`${m.id}-${m.peso_pct}-${m.monto_bono}-${m.umbrales.length}`} m={m} esRrhh={esRrhh} />
        ))}
      </ul>
      {esRrhh && (
        <div className="grid gap-2 rounded-lg border border-dashed bg-white/60 p-3">
          <div className="flex flex-wrap items-end gap-2">
            <label className="grid gap-1 text-xs text-nova-gris">
              Métrica (del catálogo)
              <select className={`${celda} min-w-52`} value={metrica} onChange={(e) => setMetrica(e.target.value)}>
                <option value="">Elegí…</option>
                {disponibles.map((m) => (
                  <option key={m.id} value={m.id}>
                    {m.nombre}
                  </option>
                ))}
                <option value={NUEVA}>+ Nueva métrica…</option>
              </select>
            </label>
            {metrica === NUEVA && (
              <>
                <label className="grid gap-1 text-xs text-nova-gris">
                  Nombre de la nueva métrica
                  <input className={`${celda} w-52`} value={nombreNueva} onChange={(e) => setNombreNueva(e.target.value)} />
                </label>
                <Button size="sm" variant="outline" disabled={crearEnCatalogo.isPending} onClick={crearMetrica}>
                  Crear en el catálogo
                </Button>
              </>
            )}
            <label className="grid gap-1 text-xs text-nova-gris">
              Peso (%)
              <input className={`${celda} w-24`} value={peso} onChange={(e) => setPeso(e.target.value)} />
            </label>
            <label className="grid gap-1 text-xs text-nova-gris">
              Monto del bono (Bs)
              <input className={`${celda} w-32`} value={monto} onChange={(e) => setMonto(e.target.value)} />
            </label>
            <Button size="sm" disabled={asignar.isPending} onClick={agregar}>
              <Plus aria-hidden /> Agregar métrica
            </Button>
          </div>
          {aviso && (
            <p className="text-xs text-error" role="alert">
              {aviso}
            </p>
          )}
          {asignar.error ? <ErrorApi error={asignar.error} /> : null}
          {crearEnCatalogo.error ? <ErrorApi error={crearEnCatalogo.error} /> : null}
        </div>
      )}
    </div>
  )
}

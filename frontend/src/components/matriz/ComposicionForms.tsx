import { useState } from 'react'

import { type CargoCfg, type FilaMatriz, type ServicioCfg, useConfigLista, useGuardarComposicion } from '@/api'
import { ErrorApi } from '@/components/ErrorApi'
import { Button } from '@/components/ui/button'

const MONTO = /^\d{1,9}(\.\d{1,2})?$/
const entrada = 'h-9 rounded-lg border border-input bg-white px-2.5 text-sm tabular-nums'

/** Valida haberes: diurno obligatorio, nocturno opcional; ambos montos no negativos (2 decimales). */
function haberes(diurno: string, nocturno: string): string | undefined {
  if (!MONTO.test(diurno.trim())) return 'Haber básico diurno: monto con hasta 2 decimales.'
  if (nocturno.trim() && !MONTO.test(nocturno.trim())) return 'Haber básico nocturno: monto con hasta 2 decimales.'
  return undefined
}

/** Alta de una fila de servicio_cargos (servicio + cargo + haberes). */
export function NuevaComposicion({ cerrar }: { cerrar: () => void }) {
  const servicios = useConfigLista<ServicioCfg>('servicios')
  const cargos = useConfigLista<CargoCfg>('cargos')
  const guardar = useGuardarComposicion()
  const [servicio, setServicio] = useState('')
  const [cargo, setCargo] = useState('')
  const [diurno, setDiurno] = useState('')
  const [nocturno, setNocturno] = useState('')
  const [aviso, setAviso] = useState<string>()

  const enviar = (e: React.FormEvent) => {
    e.preventDefault()
    const msg = !servicio ? 'Elegí el servicio.' : !cargo ? 'Elegí el cargo.' : haberes(diurno, nocturno)
    setAviso(msg)
    if (msg) return
    guardar.mutate(
      {
        body: {
          servicio_id: servicio,
          cargo_id: cargo,
          haber_basico_diurno: diurno.trim(),
          haber_basico_nocturno: nocturno.trim() || null,
        },
      },
      { onSuccess: cerrar },
    )
  }

  return (
    <form onSubmit={enviar} className="grid gap-3 rounded-xl border border-nova-azul/30 bg-card p-4 shadow-nova-md" aria-label="Nueva composición">
      <p className="font-semibold text-nova-titulo">Nueva composición</p>
      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        <label className="grid gap-1 text-xs text-nova-gris">
          Servicio *
          <select className={entrada} value={servicio} onChange={(e) => setServicio(e.target.value)}>
            <option value="">Elegí…</option>
            {(servicios.data ?? [])
              .filter((s) => s.activo)
              .map((s) => (
                <option key={s.id} value={s.id}>
                  {s.codigo ? `${s.codigo} · ${s.nombre}` : s.nombre}
                </option>
              ))}
          </select>
        </label>
        <label className="grid gap-1 text-xs text-nova-gris">
          Cargo *
          <select className={entrada} value={cargo} onChange={(e) => setCargo(e.target.value)}>
            <option value="">Elegí…</option>
            {(cargos.data ?? []).map((c) => (
              <option key={c.id} value={c.id}>
                {c.nombre}
              </option>
            ))}
          </select>
        </label>
        <label className="grid gap-1 text-xs text-nova-gris">
          Haber básico diurno (Bs) *
          <input className={entrada} inputMode="decimal" value={diurno} onChange={(e) => setDiurno(e.target.value)} />
        </label>
        <label className="grid gap-1 text-xs text-nova-gris">
          Haber básico nocturno (Bs)
          <input className={entrada} inputMode="decimal" value={nocturno} onChange={(e) => setNocturno(e.target.value)} />
        </label>
      </div>
      {aviso && (
        <p className="text-sm text-error" role="alert">
          {aviso}
        </p>
      )}
      {guardar.error ? <ErrorApi error={guardar.error} /> : null}
      <div className="flex justify-end gap-2">
        <Button type="button" variant="outline" onClick={cerrar}>
          Cancelar
        </Button>
        <Button type="submit" disabled={guardar.isPending}>
          Crear composición
        </Button>
      </div>
    </form>
  )
}

/** Edición de los haberes de una fila de la matriz. */
export function EditarHaberes({ fila, cerrar }: { fila: FilaMatriz; cerrar: () => void }) {
  const guardar = useGuardarComposicion()
  const [diurno, setDiurno] = useState(fila.haber_basico_diurno)
  const [nocturno, setNocturno] = useState(fila.haber_basico_nocturno ?? '')
  const [aviso, setAviso] = useState<string>()

  const enviar = (e: React.FormEvent) => {
    e.preventDefault()
    const msg = haberes(diurno, nocturno)
    setAviso(msg)
    if (msg) return
    guardar.mutate(
      { id: fila.servicio_cargo_id, body: { haber_basico_diurno: diurno.trim(), haber_basico_nocturno: nocturno.trim() || null } },
      { onSuccess: cerrar },
    )
  }

  return (
    <form onSubmit={enviar} className="flex flex-wrap items-end gap-3" aria-label="Editar haberes">
      <label className="grid gap-1 text-xs text-nova-gris">
        Haber básico diurno (Bs)
        <input className={`${entrada} w-40`} inputMode="decimal" value={diurno} onChange={(e) => setDiurno(e.target.value)} />
      </label>
      <label className="grid gap-1 text-xs text-nova-gris">
        Haber básico nocturno (Bs)
        <input className={`${entrada} w-40`} inputMode="decimal" value={nocturno} onChange={(e) => setNocturno(e.target.value)} />
      </label>
      <Button type="button" variant="ghost" onClick={cerrar}>
        Cancelar
      </Button>
      <Button type="submit" disabled={guardar.isPending}>
        Guardar haberes
      </Button>
      {aviso && (
        <p className="w-full text-sm text-error" role="alert">
          {aviso}
        </p>
      )}
      {guardar.error ? (
        <div className="w-full">
          <ErrorApi error={guardar.error} />
        </div>
      ) : null}
    </form>
  )
}

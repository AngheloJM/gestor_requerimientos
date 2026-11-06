import { useState } from 'react'

import { type CargoCfg, type FilaMatriz, type ServicioCfg, useConfigLista, useGuardarComposicion } from '@/api'
import { ErrorApi } from '@/components/ErrorApi'
import { Button } from '@/components/ui/button'

const MONTO = /^\d{1,9}(\.\d{1,2})?$/
const entrada = 'h-9 rounded-lg border border-input bg-white px-2.5 text-sm tabular-nums'

const ENTERO = /^\d{1,6}$/

/** Valida haberes: diurno obligatorio, nocturno opcional; ambos montos no negativos (2 decimales). */
function haberes(diurno: string, nocturno: string): string | undefined {
  if (!MONTO.test(diurno.trim())) return 'Haber básico diurno: monto con hasta 2 decimales.'
  if (nocturno.trim() && !MONTO.test(nocturno.trim())) return 'Haber básico nocturno: monto con hasta 2 decimales.'
  return undefined
}

/** Cantidades opcionales (enteros ≥ 0). */
function cantidades(mujeres: string, hombres: string): string | undefined {
  if (mujeres.trim() && !ENTERO.test(mujeres.trim())) return 'Cantidad de mujeres: número entero.'
  if (hombres.trim() && !ENTERO.test(hombres.trim())) return 'Cantidad de hombres: número entero.'
  return undefined
}
const entero = (v: string) => (v.trim() ? Number(v.trim()) : null)

const DIAS = /^\d{1,4}$/
type Tiempos = { capacitacion: string; ojt: string; ajustada: string }
const TIEMPOS_VACIOS: Tiempos = { capacitacion: '', ojt: '', ajustada: '' }

/** Tiempos opcionales en días enteros. */
function tiemposValidos(t: Tiempos): string | undefined {
  for (const [v, n] of [
    [t.capacitacion, 'de capacitación'],
    [t.ojt, 'de OJT'],
    [t.ajustada, 'de capacitación ajustada'],
  ]) {
    if (v.trim() && !DIAS.test(v.trim())) return `Tiempo ${n}: número entero de días.`
  }
  return undefined
}
const tiemposBody = (t: Tiempos) => ({
  tiempo_capacitacion_dias: entero(t.capacitacion),
  tiempo_ojt_dias: entero(t.ojt),
  tiempo_capacitacion_ajustada_dias: entero(t.ajustada),
})

function CamposTiempos({ t, set, ancho = '' }: { t: Tiempos; set: (t: Tiempos) => void; ancho?: string }) {
  const campo = (k: keyof Tiempos, etiqueta: string) => (
    <label className="grid gap-1 text-xs text-nova-gris">
      {etiqueta}
      <input className={`${entrada} ${ancho}`} inputMode="numeric" value={t[k]} onChange={(e) => set({ ...t, [k]: e.target.value })} />
    </label>
  )
  return (
    <>
      {campo('capacitacion', 'Tiempo de capacitación (días)')}
      {campo('ojt', 'Tiempo de OJT (días)')}
      {campo('ajustada', 'Tiempo de capacitación ajustada (días)')}
    </>
  )
}

function CamposCantidades({
  mujeres,
  hombres,
  setMujeres,
  setHombres,
  ancho = '',
}: {
  mujeres: string
  hombres: string
  setMujeres: (v: string) => void
  setHombres: (v: string) => void
  ancho?: string
}) {
  return (
    <>
      <label className="grid gap-1 text-xs text-nova-gris">
        Cantidad de mujeres
        <input className={`${entrada} ${ancho}`} inputMode="numeric" value={mujeres} onChange={(e) => setMujeres(e.target.value)} />
      </label>
      <label className="grid gap-1 text-xs text-nova-gris">
        Cantidad de hombres
        <input className={`${entrada} ${ancho}`} inputMode="numeric" value={hombres} onChange={(e) => setHombres(e.target.value)} />
      </label>
    </>
  )
}

/**
 * Alta de una fila de servicio_cargos (servicio + cargo + haberes + cantidades).
 * Con `servicioId`, el servicio queda fijo (formulario dentro del Servicio).
 */
export function NuevaComposicion({
  cerrar,
  servicioId,
  excluirCargos = [],
  onCreada,
}: {
  cerrar: () => void
  servicioId?: string
  excluirCargos?: string[]
  onCreada?: (servicioCargoId: string) => void
}) {
  const servicios = useConfigLista<ServicioCfg>('servicios')
  const cargos = useConfigLista<CargoCfg>('cargos')
  const guardar = useGuardarComposicion()
  const [servicio, setServicio] = useState(servicioId ?? '')
  const [cargo, setCargo] = useState('')
  const [diurno, setDiurno] = useState('')
  const [nocturno, setNocturno] = useState('')
  const [mujeres, setMujeres] = useState('')
  const [hombres, setHombres] = useState('')
  const [tiempos, setTiempos] = useState<Tiempos>(TIEMPOS_VACIOS)
  const [aviso, setAviso] = useState<string>()

  const enviar = (e: React.FormEvent) => {
    e.preventDefault()
    const msg = !servicio
      ? 'Seleccione el servicio.'
      : !cargo
        ? 'Seleccione el cargo.'
        : (haberes(diurno, nocturno) ?? cantidades(mujeres, hombres) ?? tiemposValidos(tiempos))
    setAviso(msg)
    if (msg) return
    guardar.mutate(
      {
        body: {
          servicio_id: servicio,
          cargo_id: cargo,
          haber_basico_diurno: diurno.trim(),
          haber_basico_nocturno: nocturno.trim() || null,
          cantidad_mujeres: entero(mujeres),
          cantidad_hombres: entero(hombres),
          ...tiemposBody(tiempos),
        },
      },
      {
        onSuccess: (r) => {
          cerrar()
          onCreada?.(r.id)
        },
      },
    )
  }

  return (
    <form onSubmit={enviar} className="grid gap-3 rounded-xl border border-nova-azul/30 bg-card p-4 shadow-nova-md" aria-label="Agregar cargo">
      <p className="font-semibold text-nova-titulo">{servicioId ? 'Agregar cargo al servicio' : 'Nueva composición'}</p>
      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {!servicioId && (
          <label className="grid gap-1 text-xs text-nova-gris">
            Servicio *
            <select className={entrada} value={servicio} onChange={(e) => setServicio(e.target.value)}>
              <option value="">Seleccione una opción</option>
              {(servicios.data ?? [])
                .filter((s) => s.activo)
                .map((s) => (
                  <option key={s.id} value={s.id}>
                    {s.codigo ? `${s.codigo} · ${s.nombre}` : s.nombre}
                  </option>
                ))}
            </select>
          </label>
        )}
        <label className="grid gap-1 text-xs text-nova-gris">
          Cargo *
          <select className={entrada} value={cargo} onChange={(e) => setCargo(e.target.value)}>
            <option value="">Seleccione una opción</option>
            {(cargos.data ?? []).filter((c) => !excluirCargos.includes(c.id)).map((c) => (
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
        <CamposCantidades mujeres={mujeres} hombres={hombres} setMujeres={setMujeres} setHombres={setHombres} />
        <CamposTiempos t={tiempos} set={setTiempos} />
      </div>
      {servicioId && <p className="text-xs text-nova-gris">Después de guardar se abren las métricas y bonos de este cargo.</p>}
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
          {servicioId ? 'Agregar cargo' : 'Crear composición'}
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
  const [mujeres, setMujeres] = useState(fila.cantidad_mujeres?.toString() ?? '')
  const [hombres, setHombres] = useState(fila.cantidad_hombres?.toString() ?? '')
  const [tiempos, setTiempos] = useState<Tiempos>({
    capacitacion: fila.tiempo_capacitacion_dias?.toString() ?? '',
    ojt: fila.tiempo_ojt_dias?.toString() ?? '',
    ajustada: fila.tiempo_capacitacion_ajustada_dias?.toString() ?? '',
  })
  const [aviso, setAviso] = useState<string>()

  const enviar = (e: React.FormEvent) => {
    e.preventDefault()
    const msg = haberes(diurno, nocturno) ?? cantidades(mujeres, hombres) ?? tiemposValidos(tiempos)
    setAviso(msg)
    if (msg) return
    guardar.mutate(
      {
        id: fila.servicio_cargo_id,
        body: {
          haber_basico_diurno: diurno.trim(),
          haber_basico_nocturno: nocturno.trim() || null,
          cantidad_mujeres: entero(mujeres),
          cantidad_hombres: entero(hombres),
          ...tiemposBody(tiempos),
        },
      },
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
      <CamposCantidades mujeres={mujeres} hombres={hombres} setMujeres={setMujeres} setHombres={setHombres} ancho="w-32" />
      <CamposTiempos t={tiempos} set={setTiempos} ancho="w-40" />
      <Button type="button" variant="ghost" onClick={cerrar}>
        Cancelar
      </Button>
      <Button type="submit" disabled={guardar.isPending}>
        Guardar
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

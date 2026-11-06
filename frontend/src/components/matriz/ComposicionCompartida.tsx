import { AlertCircle, Pencil, Plus, Trash2 } from 'lucide-react'
import { useState } from 'react'

import { type CargoCfg, type ComposicionCompartida as Compartida, useCompartida, useCompartidas, useConfigLista, useGuardarCompartida } from '@/api'
import { ErrorApi } from '@/components/ErrorApi'
import { Button } from '@/components/ui/button'
import { Skeleton } from '@/components/ui/skeleton'
import { formatMonto } from '@/lib/format'

const MONTO = /^\d{1,9}(\.\d{1,2})?$/
const entrada = 'h-9 w-full rounded-lg border border-input bg-white px-2.5 text-sm tabular-nums disabled:bg-muted disabled:opacity-60'
const AVISO_NO_COMPARTIDO = 'Este apartado es solo para cargos que no pertenecen a un solo servicio.'

type Par = { centro: string; monto: string }

/** % de cada monto sobre el total (solo para mostrar mientras se edita; la base guarda montos). */
function porcentajes(pares: Par[]): (number | null)[] {
  const nums = pares.map((p) => (MONTO.test(p.monto.trim()) ? Number(p.monto) : null))
  const total = nums.reduce<number>((a, n) => a + (n ?? 0), 0)
  return nums.map((n) => (n == null || total === 0 ? null : (n * 100) / total))
}

function Formulario({ cargoInicial, cerrar }: { cargoInicial?: string; cerrar: () => void }) {
  const cargos = useConfigLista<CargoCfg>('cargos')
  const [cargo, setCargo] = useState(cargoInicial ?? '')
  const actual = useCompartida(cargo)
  const guardar = useGuardarCompartida()
  const [pares, setPares] = useState<Par[] | null>(null)
  const [aviso, setAviso] = useState<string>()

  // Carga inicial de la distribución existente al elegir el cargo.
  const datos = actual.data
  const filas: Par[] =
    pares ??
    (datos && datos.distribucion.length > 0
      ? datos.distribucion.map((d) => ({ centro: d.centro_costo_id, monto: d.monto }))
      : [
          { centro: '', monto: '' },
          { centro: '', monto: '' },
        ])
  const set = (i: number, cambio: Partial<Par>) => setPares(filas.map((p, j) => (j === i ? { ...p, ...cambio } : p)))
  const pct = porcentajes(filas)
  const total = filas.reduce((a, p) => a + (MONTO.test(p.monto.trim()) ? Number(p.monto) : 0), 0)
  const bloqueado = !!datos && !datos.compartido

  const enviar = (e: React.FormEvent) => {
    e.preventDefault()
    const usados = filas.map((p) => p.centro)
    const msg = !cargo
      ? 'Seleccione el cargo.'
      : bloqueado
        ? AVISO_NO_COMPARTIDO
        : filas.length < 2
          ? 'Indique al menos dos centros de costo.'
          : filas.some((p) => !p.centro)
            ? 'Seleccione el centro de costo de cada fila.'
            : new Set(usados).size !== usados.length
              ? 'Cada centro de costo puede aparecer una sola vez.'
              : filas.some((p) => !MONTO.test(p.monto.trim()) || Number(p.monto) <= 0)
                ? 'Cada monto debe ser mayor a cero, con hasta 2 decimales.'
                : undefined
    setAviso(msg)
    if (msg) return
    guardar.mutate({ cargoId: cargo, centros: filas.map((p) => ({ centro_costo_id: p.centro, monto: p.monto.trim() })) }, { onSuccess: cerrar })
  }

  return (
    <form onSubmit={enviar} className="grid gap-4 rounded-xl border border-nova-azul/30 bg-card p-4 shadow-nova-md" aria-label="Composición de cargo compartido">
      <p className="font-semibold text-nova-titulo">Composición de cargo compartido</p>
      <label className="grid max-w-md gap-1 text-xs text-nova-gris">
        Cargo *
        <select
          className={entrada}
          value={cargo}
          disabled={!!cargoInicial}
          onChange={(e) => {
            setCargo(e.target.value)
            setPares(null)
            setAviso(undefined)
          }}
        >
          <option value="">Seleccione una opción</option>
          {(cargos.data ?? []).map((c) => (
            <option key={c.id} value={c.id}>
              {c.nombre}
            </option>
          ))}
        </select>
      </label>

      {cargo && actual.isPending && <Skeleton className="h-24 rounded-lg" />}
      {actual.error ? <ErrorApi error={actual.error} /> : null}

      {bloqueado && (
        <p className="flex gap-2 rounded-lg border border-error/40 bg-error/10 p-3 text-sm font-medium text-error" role="alert">
          <AlertCircle className="size-4 shrink-0" aria-hidden />
          <span>
            {AVISO_NO_COMPARTIDO}{' '}
            <span className="font-normal">
              {datos.centros_posibles.length === 1
                ? `Este cargo solo tiene un centro de costo (${datos.centros_posibles[0].nombre}): su composición se arma dentro del servicio.`
                : 'Este cargo no está asignado a ningún servicio activo con centro de costo.'}
            </span>
          </span>
        </p>
      )}

      {datos && datos.compartido && (
        <div className="grid gap-2">
          {filas.map((p, i) => (
            <div key={i} className="grid items-end gap-2 sm:grid-cols-[minmax(0,1fr)_12rem_6rem_auto]">
              <label className="grid gap-1 text-xs text-nova-gris">
                Centro de costo
                <select className={entrada} value={p.centro} onChange={(e) => set(i, { centro: e.target.value })}>
                  <option value="">Seleccione una opción</option>
                  {datos.centros_posibles.map((c) => (
                    <option key={c.id} value={c.id}>
                      {c.nombre}
                    </option>
                  ))}
                </select>
              </label>
              <label className="grid gap-1 text-xs text-nova-gris">
                Monto (Bs)
                <input className={entrada} inputMode="decimal" value={p.monto} onChange={(e) => set(i, { monto: e.target.value })} />
              </label>
              <p className="h-9 rounded-lg bg-nova-hielo px-2.5 text-right text-sm leading-9 font-semibold tabular-nums text-nova-azul" aria-label="Porcentaje del total">
                {pct[i] == null ? '—' : `${pct[i]!.toFixed(2)} %`}
              </p>
              <Button
                type="button"
                variant="ghost"
                size="icon"
                disabled={filas.length <= 2}
                onClick={() => setPares(filas.filter((_, j) => j !== i))}
                aria-label="Quitar centro de costo"
              >
                <Trash2 />
              </Button>
            </div>
          ))}
          <div className="flex flex-wrap items-center justify-between gap-2">
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={filas.length >= datos.centros_posibles.length}
              onClick={() => setPares([...filas, { centro: '', monto: '' }])}
            >
              <Plus aria-hidden /> Agregar centro de costo
            </Button>
            <p className="text-sm">
              <span className="text-nova-gris">Sueldo total: </span>
              <span className="font-semibold tabular-nums text-nova-titulo">Bs {formatMonto(total.toFixed(2))}</span>
            </p>
          </div>
        </div>
      )}

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
        <Button type="submit" disabled={guardar.isPending || !datos || bloqueado}>
          Guardar composición
        </Button>
      </div>
    </form>
  )
}

function Tarjeta({ c, esRrhh, editar }: { c: Compartida; esRrhh: boolean; editar: () => void }) {
  return (
    <li className="rounded-xl border bg-card p-4 shadow-nova-xs">
      <div className="flex items-center justify-between gap-2">
        <p className="font-semibold text-nova-titulo">{c.cargo_nombre}</p>
        {esRrhh && (
          <Button variant="ghost" size="sm" onClick={editar}>
            <Pencil aria-hidden /> Editar
          </Button>
        )}
      </div>
      <ul className="mt-2 grid gap-1 text-sm">
        {c.distribucion.map((d) => (
          <li key={d.centro_costo_id} className="flex justify-between gap-3">
            <span className="text-nova-texto-2">{d.centro_costo_nombre}</span>
            <span className="tabular-nums">
              Bs {formatMonto(d.monto)} <span className="ml-1 font-semibold text-nova-azul">{Number(d.porcentaje).toFixed(2)} %</span>
            </span>
          </li>
        ))}
      </ul>
      <p className="mt-2 border-t pt-2 text-right text-sm">
        <span className="text-nova-gris">Total: </span>
        <span className="font-semibold tabular-nums">Bs {formatMonto(c.total)}</span>
      </p>
    </li>
  )
}

/** Cargos compartidos entre varios centros de costo: distribución del sueldo por centro. */
export function ComposicionCompartida({ esRrhh }: { esRrhh: boolean }) {
  const lista = useCompartidas()
  const [form, setForm] = useState<{ cargo?: string } | null>(null)

  return (
    <section className="grid gap-4">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h3 className="text-lg font-semibold text-nova-titulo">Cargos compartidos entre centros de costo</h3>
          <p className="mt-1 text-sm text-nova-gris">
            Solo para cargos que no pertenecen a un solo servicio. La composición de un cargo de un solo servicio se arma dentro del
            servicio (pestaña Servicios → editar).
          </p>
        </div>
        {esRrhh && (
          <Button onClick={() => setForm(form ? null : {})} aria-expanded={!!form && !form.cargo}>
            <Plus aria-hidden /> Nueva composición
          </Button>
        )}
      </div>
      {form && <Formulario key={form.cargo ?? 'nuevo'} cargoInicial={form.cargo} cerrar={() => setForm(null)} />}
      {lista.error ? (
        <ErrorApi error={lista.error} />
      ) : lista.isPending ? (
        <Skeleton className="h-24 rounded-xl" />
      ) : lista.data.length === 0 ? (
        <p className="rounded-xl border border-dashed bg-card p-6 text-center text-sm text-nova-gris">Todavía no hay cargos compartidos.</p>
      ) : (
        <ul className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
          {lista.data.map((c) => (
            <Tarjeta key={c.cargo_id} c={c} esRrhh={esRrhh} editar={() => setForm({ cargo: c.cargo_id })} />
          ))}
        </ul>
      )}
    </section>
  )
}

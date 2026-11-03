import { Info, Pencil, Plus, Search, SlidersHorizontal } from 'lucide-react'
import { Fragment, useMemo, useState } from 'react'
import { useSearchParams } from 'react-router-dom'

import { type FilaMatriz, useMatrizServicio, useParametros } from '@/api'
import { EditarHaberes, NuevaComposicion } from '@/components/matriz/ComposicionForms'
import { MetricasEditor } from '@/components/matriz/MetricasEditor'
import { Button } from '@/components/ui/button'
import { ErrorApi } from '@/components/ErrorApi'
import { SolicitarCambio } from '@/components/solicitudes/SolicitarCambio'
import { Input } from '@/components/ui/input'
import { Skeleton } from '@/components/ui/skeleton'
import { formatMonto } from '@/lib/format'
import { SALARIO_MINIMO_NACIONAL, formatParametro } from '@/lib/parametros'

/** Orden obligatorio de columnas (decisión de negocio): Centro de Costo → Cliente → Servicio → Cargo. */
const centroTexto = (f: FilaMatriz) =>
  f.centro_costo ? `${f.centro_costo.codigo} · ${f.centro_costo.nombre}` : 'Sin centro de costo'
const servicioTexto = (f: FilaMatriz) => (f.servicio.codigo ? `${f.servicio.codigo} · ${f.servicio.nombre}` : f.servicio.nombre)

const bs = (m: string | null) => (m == null ? '—' : `Bs ${formatMonto(m)}`)

function Filtro({
  id,
  etiqueta,
  valor,
  opciones,
  onChange,
}: {
  id: string
  etiqueta: string
  valor: string
  opciones: [string, string][]
  onChange: (v: string) => void
}) {
  return (
    <label htmlFor={id} className="grid gap-1 text-xs text-nova-gris">
      {etiqueta}
      <select id={id} value={valor} onChange={(e) => onChange(e.target.value)} className="h-9 rounded-lg border border-input bg-white px-2.5 text-sm text-nova-titulo">
        <option value="">Todos</option>
        {opciones.map(([v, t]) => (
          <option key={v} value={v}>
            {t}
          </option>
        ))}
      </select>
    </label>
  )
}

/** Composición salarial: tabla plana (lectura) + alta/edición de haberes y métricas (RR.HH.). */
export function ComposicionSalarial({ esRrhh }: { esRrhh: boolean }) {
  const { data, isPending, error } = useMatrizServicio()
  const parametros = useParametros()
  const [centro, setCentro] = useState('')
  const [cliente, setCliente] = useState('')
  // "Ir a corregir" del tablero llega con ?servicio=<id>.
  const [params] = useSearchParams()
  const [servicio, setServicio] = useState(params.get('servicio') ?? '')
  const [buscar, setBuscar] = useState('')

  const opciones = useMemo(() => {
    const unico = (pares: [string, string][]) => [...new Map(pares).entries()]
    const filas = data ?? []
    return {
      centros: unico(filas.map((f) => [f.centro_costo?.id ?? '-', centroTexto(f)])),
      clientes: unico(filas.map((f) => [f.cliente?.id ?? '-', f.cliente?.nombre ?? 'Sin cliente'])),
      servicios: unico(filas.map((f) => [f.servicio.id, servicioTexto(f)])),
    }
  }, [data])

  const filtradas = (data ?? []).filter((f) => {
    const q = buscar.trim().toLowerCase()
    return (
      (!centro || (f.centro_costo?.id ?? '-') === centro) &&
      (!cliente || (f.cliente?.id ?? '-') === cliente) &&
      (!servicio || f.servicio.id === servicio) &&
      (!q ||
        [centroTexto(f), f.cliente?.nombre, f.proyecto?.nombre, servicioTexto(f), f.cargo.nombre].some((v) => v?.toLowerCase().includes(q)))
    )
  })
  // Fila extendida bajo un registro: solicitar cambio, editar haberes o métricas.
  const [abierto, setAbierto] = useState<{ id: string; modo: 'solicitar' | 'editar' | 'metricas' } | null>(null)
  const solicitando = abierto?.modo === 'solicitar' ? abierto.id : null
  const setSolicitando = (id: string | null) => setAbierto(id ? { id, modo: 'solicitar' } : null)
  const alternar = (id: string, modo: 'editar' | 'metricas') =>
    setAbierto(abierto?.id === id && abierto.modo === modo ? null : { id, modo })
  const [nueva, setNueva] = useState(false)
  const [enviada, setEnviada] = useState<string | null>(null)
  const cargosDe = (servicioId: string) =>
    (data ?? []).filter((x) => x.servicio.id === servicioId).map((x) => ({ id: x.cargo.id, nombre: x.cargo.nombre }))
  const smn = parametros.data?.vigentes.find((v) => v.parametro === SALARIO_MINIMO_NACIONAL)

  return (
    <div className="grid gap-5">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h3 className="text-lg font-semibold text-nova-titulo">Composición salarial</h3>
          <p className="mt-1 text-sm text-nova-gris">Centro de costo → Cliente → Servicio → Cargo, con sueldos, bonos y total ganado.</p>
        </div>
        {esRrhh && (
          <Button onClick={() => setNueva(!nueva)} aria-expanded={nueva}>
            <Plus aria-hidden /> Nueva composición
          </Button>
        )}
        {smn && (
          <p className="rounded-lg border bg-card px-3 py-2 text-sm shadow-nova-xs">
            <span className="text-nova-gris">Salario Mínimo Nacional vigente: </span>
            <span className="font-semibold text-nova-titulo">{formatParametro(smn.parametro, smn.valor)}</span>
          </p>
        )}
      </div>

      {nueva && <NuevaComposicion cerrar={() => setNueva(false)} />}

      <div className="grid gap-3 rounded-xl border bg-card p-4 shadow-nova-xs sm:grid-cols-2 lg:grid-cols-4">
        <Filtro id="f-centro" etiqueta="Centro de costo" valor={centro} opciones={opciones.centros} onChange={setCentro} />
        <Filtro id="f-cliente" etiqueta="Cliente" valor={cliente} opciones={opciones.clientes} onChange={setCliente} />
        <Filtro id="f-servicio" etiqueta="Servicio" valor={servicio} opciones={opciones.servicios} onChange={setServicio} />
        <label className="grid gap-1 text-xs text-nova-gris">
          Buscar
          <span className="relative">
            <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2" aria-hidden />
            <Input className="h-9 bg-white pl-8 text-sm" value={buscar} onChange={(e) => setBuscar(e.target.value)} placeholder="Centro, cliente, proyecto, servicio o cargo" />
          </span>
        </label>
      </div>

      <p className="flex gap-2 text-xs text-nova-gris">
        <Info className="size-3.5 shrink-0" aria-hidden />
        Total ganado = haber básico (diurno + nocturno) + bonos configurados, sin ajustar por cumplimiento real. {filtradas.length} cargo(s).
      </p>

      {error ? (
        <ErrorApi error={error} />
      ) : isPending ? (
        <Skeleton className="h-64 rounded-xl" />
      ) : filtradas.length === 0 ? (
        <div className="rounded-xl border border-dashed bg-card p-10 text-center text-sm text-nova-gris">Sin resultados.</div>
      ) : (
        // Una sola tabla continua (tipo planilla): una fila por registro de servicio_cargos.
        <div className="max-h-[70vh] overflow-auto rounded-xl border bg-card shadow-nova-sm">
          <table className="w-full min-w-[1180px] border-separate border-spacing-0 text-sm">
            <thead className="sticky top-0 z-10 bg-nova-hielo text-left text-xs tracking-wide text-nova-gris uppercase">
              <tr>
                {['Centro de costo', 'Cliente', 'Servicio', 'Cargo', 'Proyecto'].map((h) => (
                  <th key={h} className="border-b px-3 py-2.5 font-medium whitespace-nowrap">
                    {h}
                  </th>
                ))}
                <th className="border-b px-3 py-2.5 text-right font-medium whitespace-nowrap">Haber diurno</th>
                <th className="border-b px-3 py-2.5 text-right font-medium whitespace-nowrap">Haber nocturno</th>
                <th className="border-b px-3 py-2.5 font-medium whitespace-nowrap">Bonos configurados</th>
                <th className="border-b px-3 py-2.5 text-right font-medium whitespace-nowrap">Total ganado</th>
                <th className="border-b px-3 py-2.5" aria-label="Acciones" />
              </tr>
            </thead>
            <tbody>
              {filtradas.map((f) => (
                <Fragment key={f.servicio_cargo_id}>
                  <tr className="align-top hover:bg-nova-hielo/30">
                    <td className="border-b px-3 py-2 whitespace-nowrap text-nova-texto-2">{centroTexto(f)}</td>
                    <td className="border-b px-3 py-2 whitespace-nowrap">{f.cliente?.nombre ?? '—'}</td>
                    <td className="border-b px-3 py-2 whitespace-nowrap">{servicioTexto(f)}</td>
                    <td className="border-b px-3 py-2 font-medium whitespace-nowrap text-nova-titulo">{f.cargo.nombre}</td>
                    <td className="border-b px-3 py-2 whitespace-nowrap text-nova-texto-2">{f.proyecto?.nombre ?? '—'}</td>
                    <td className="border-b px-3 py-2 text-right whitespace-nowrap tabular-nums">{bs(f.haber_basico_diurno)}</td>
                    <td className="border-b px-3 py-2 text-right whitespace-nowrap tabular-nums">{bs(f.haber_basico_nocturno)}</td>
                    <td className="border-b px-3 py-2">
                      {f.metricas.length === 0 ? (
                        <span className="text-nova-gris">—</span>
                      ) : (
                        <ul className="grid gap-0.5">
                          {f.metricas.map((m) => (
                            <li key={m.id} className="flex justify-between gap-3 whitespace-nowrap">
                              <span>{m.nombre}</span>
                              <span className="tabular-nums text-nova-texto-2">{bs(m.monto_bono)}</span>
                            </li>
                          ))}
                        </ul>
                      )}
                    </td>
                    <td className="border-b px-3 py-2 text-right font-semibold whitespace-nowrap tabular-nums text-nova-azul">{bs(f.total_ganado)}</td>
                    <td className="border-b px-3 py-1.5 text-right whitespace-nowrap">
                      <div className="flex items-center justify-end gap-1 [&>span]:ml-0">
                        {enviada === f.servicio_cargo_id && (
                          <span className="mr-2 text-xs text-exito" role="status">
                            Solicitud enviada
                          </span>
                        )}
                        {esRrhh && (
                          <>
                            <Button
                              variant="ghost"
                              size="sm"
                              aria-expanded={abierto?.id === f.servicio_cargo_id && abierto.modo === 'editar'}
                              onClick={() => alternar(f.servicio_cargo_id, 'editar')}
                            >
                              <Pencil aria-hidden /> Editar
                            </Button>
                            <Button
                              variant="ghost"
                              size="sm"
                              aria-expanded={abierto?.id === f.servicio_cargo_id && abierto.modo === 'metricas'}
                              onClick={() => alternar(f.servicio_cargo_id, 'metricas')}
                            >
                              <SlidersHorizontal aria-hidden /> Métricas
                            </Button>
                          </>
                        )}
                        <SolicitarCambio
                          servicioId={f.servicio.id}
                          cargos={cargosDe(f.servicio.id)}
                          abierto={solicitando === f.servicio_cargo_id}
                          onAbrir={() => setSolicitando(f.servicio_cargo_id)}
                          onCerrar={() => setSolicitando(null)}
                          soloBoton
                        />
                      </div>
                    </td>
                  </tr>
                  {solicitando === f.servicio_cargo_id && (
                    <tr>
                      <td colSpan={10} className="border-b bg-nova-hielo/30 px-3 py-2">
                        <p className="text-xs text-nova-gris">
                          Solicitar cambio · {servicioTexto(f)} · {f.cargo.nombre}
                        </p>
                        <SolicitarCambio
                          servicioId={f.servicio.id}
                          cargos={cargosDe(f.servicio.id)}
                          cargoInicial={f.cargo.id}
                          abierto
                          onCerrar={() => setSolicitando(null)}
                          onEnviada={() => setEnviada(f.servicio_cargo_id)}
                        />
                      </td>
                    </tr>
                  )}
                  {abierto?.id === f.servicio_cargo_id && abierto.modo !== 'solicitar' && (
                    <tr>
                      <td colSpan={10} className="border-b bg-nova-hielo/30 px-3 py-3">
                        <p className="sticky left-3 mb-2 text-xs text-nova-gris">
                          {abierto.modo === 'editar' ? 'Editar haberes' : 'Métricas y bonos'} · {servicioTexto(f)} · {f.cargo.nombre}
                        </p>
                        {/* Pegado al borde visible de la planilla (que se desplaza en horizontal). */}
                        <div className="sticky left-3 max-w-4xl">
                          {abierto.modo === 'editar' ? (
                            <EditarHaberes fila={f} cerrar={() => setAbierto(null)} />
                          ) : (
                            <MetricasEditor servicioCargoId={f.servicio_cargo_id} esRrhh={esRrhh} />
                          )}
                        </div>
                      </td>
                    </tr>
                  )}
                </Fragment>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}

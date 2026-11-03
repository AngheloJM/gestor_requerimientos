import { Info, Search } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useSearchParams } from 'react-router-dom'

import { type FilaMatriz, useMatrizServicio, useParametros } from '@/api'
import { ErrorApi } from '@/components/ErrorApi'
import { SolicitarCambio } from '@/components/solicitudes/SolicitarCambio'
import { Input } from '@/components/ui/input'
import { Skeleton } from '@/components/ui/skeleton'
import { formatMonto } from '@/lib/format'
import { SALARIO_MINIMO_NACIONAL, formatParametro } from '@/lib/parametros'

/** Orden obligatorio (decisión de negocio): Centro de Costo → Cliente → Servicio → Cargo. */
interface GrupoServicio {
  clave: string
  centro: string
  cliente: string
  servicio: string
  filas: FilaMatriz[]
}

const centroTexto = (f: FilaMatriz) =>
  f.centro_costo ? `${f.centro_costo.codigo} · ${f.centro_costo.nombre}` : 'Sin centro de costo'
const servicioTexto = (f: FilaMatriz) => (f.servicio.codigo ? `${f.servicio.codigo} · ${f.servicio.nombre}` : f.servicio.nombre)

function agrupar(filas: FilaMatriz[]): GrupoServicio[] {
  // La API ya devuelve el orden CC → Cliente → Servicio → Cargo; se agrupa respetándolo.
  const grupos: GrupoServicio[] = []
  for (const f of filas) {
    const clave = `${f.centro_costo?.id ?? '-'}|${f.cliente?.id ?? '-'}|${f.servicio.id}`
    let g = grupos[grupos.length - 1]
    if (!g || g.clave !== clave) {
      g = { clave, centro: centroTexto(f), cliente: f.cliente?.nombre ?? 'Sin cliente', servicio: servicioTexto(f), filas: [] }
      grupos.push(g)
    }
    g.filas.push(f)
  }
  return grupos
}

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

export function MatrizPage() {
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
  const grupos = agrupar(filtradas)
  const smn = parametros.data?.vigentes.find((v) => v.parametro === SALARIO_MINIMO_NACIONAL)

  return (
    <div className="grid gap-5">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h2 className="text-2xl font-semibold text-nova-titulo">Matriz de servicio</h2>
          <p className="mt-1 text-sm text-nova-gris">Centro de costo → Cliente → Servicio → Cargo, con sueldos, bonos y total ganado.</p>
        </div>
        {smn && (
          <p className="rounded-lg border bg-card px-3 py-2 text-sm shadow-nova-xs">
            <span className="text-nova-gris">Salario Mínimo Nacional vigente: </span>
            <span className="font-semibold text-nova-titulo">{formatParametro(smn.parametro, smn.valor)}</span>
          </p>
        )}
      </div>

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
      ) : grupos.length === 0 ? (
        <div className="rounded-xl border border-dashed bg-card p-10 text-center text-sm text-nova-gris">Sin resultados.</div>
      ) : (
        <div className="grid gap-4">
          {grupos.map((g) => (
            <section key={g.clave} className="overflow-hidden rounded-xl border bg-card shadow-nova-sm">
              <header className="flex flex-wrap items-baseline gap-x-2 gap-y-1 border-b bg-nova-hielo/50 px-4 py-3 text-sm">
                <span className="text-nova-gris">{g.centro}</span>
                <span className="text-nova-gris" aria-hidden>
                  →
                </span>
                <span className="text-nova-texto-2">{g.cliente}</span>
                <span className="text-nova-gris" aria-hidden>
                  →
                </span>
                <span className="font-semibold text-nova-titulo">{g.servicio}</span>
                {g.filas[0].proyecto && <span className="text-xs text-nova-gris">(proyecto {g.filas[0].proyecto.nombre})</span>}
                <SolicitarCambio servicioId={g.filas[0].servicio.id} cargos={g.filas.map((f) => ({ id: f.cargo.id, nombre: f.cargo.nombre }))} />
              </header>
              <div className="overflow-x-auto">
                <table className="w-full min-w-[760px] text-sm">
                  <thead className="text-left text-xs tracking-wide text-nova-gris uppercase">
                    <tr>
                      <th className="px-4 py-2 font-medium">Cargo</th>
                      <th className="px-4 py-2 text-right font-medium">Haber diurno</th>
                      <th className="px-4 py-2 text-right font-medium">Haber nocturno</th>
                      <th className="px-4 py-2 font-medium">Bonos configurados</th>
                      <th className="px-4 py-2 text-right font-medium">Total ganado</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y">
                    {g.filas.map((f) => (
                      <tr key={f.servicio_cargo_id} className="align-top hover:bg-nova-hielo/30">
                        <td className="px-4 py-2.5 font-medium text-nova-titulo">{f.cargo.nombre}</td>
                        <td className="px-4 py-2.5 text-right tabular-nums">{bs(f.haber_basico_diurno)}</td>
                        <td className="px-4 py-2.5 text-right tabular-nums">{bs(f.haber_basico_nocturno)}</td>
                        <td className="px-4 py-2.5">
                          {f.metricas.length === 0 ? (
                            <span className="text-nova-gris">—</span>
                          ) : (
                            <ul className="grid gap-0.5">
                              {f.metricas.map((m) => (
                                <li key={m.id} className="flex justify-between gap-3">
                                  <span>{m.nombre}</span>
                                  <span className="tabular-nums text-nova-texto-2">{bs(m.monto_bono)}</span>
                                </li>
                              ))}
                            </ul>
                          )}
                        </td>
                        <td className="px-4 py-2.5 text-right font-semibold tabular-nums text-nova-azul">{bs(f.total_ganado)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </section>
          ))}
        </div>
      )}
    </div>
  )
}

import { useState } from 'react'
import { useSearchParams } from 'react-router-dom'

import {
  type BonoCatalogoCfg,
  type CargoCfg,
  type CentroCostoCfg,
  type ClienteCfg,
  type ContratoCfg,
  type MetricaCatalogoCfg,
  type ServicioCfg,
  useConfigLista,
  useMatrizServicio,
  useMe,
} from '@/api'
import { ComposicionSalarial } from '@/components/matriz/ComposicionSalarial'
import { MetricasEditor } from '@/components/matriz/MetricasEditor'
import {
  CargosSeccion,
  CatalogoBonos,
  CatalogoMetricas,
  CentrosCosto,
  ClientesYProyectos,
  Contratos,
  PendientesCompletar,
  Servicios,
} from '@/components/matriz/Secciones'
import { cn } from '@/lib/utils'

type Seccion =
  | 'clientes'
  | 'centros'
  | 'servicios'
  | 'cargos'
  | 'composicion'
  | 'contratos'
  | 'metricas'
  | 'catalogo-metricas'
  | 'catalogo-bonos'
  | 'pendientes'

/** Sección 7: elegir un servicio + cargo y editar sus métricas y escalas. */
function MetricasPorCargo({ esRrhh }: { esRrhh: boolean }) {
  const { data } = useMatrizServicio()
  const [sel, setSel] = useState('')
  return (
    <section className="grid gap-4">
      <div>
        <h3 className="text-lg font-semibold text-nova-titulo">Métricas y bonos por cargo</h3>
        <p className="text-sm text-nova-gris">Elegí una composición (servicio + cargo); la métrica se toma del catálogo de métricas.</p>
      </div>
      <select
        aria-label="Composición"
        className="h-9 max-w-xl rounded-lg border border-input bg-white px-2.5 text-sm"
        value={sel}
        onChange={(e) => setSel(e.target.value)}
      >
        <option value="">Elegí servicio y cargo…</option>
        {(data ?? []).map((f) => (
          <option key={f.servicio_cargo_id} value={f.servicio_cargo_id}>
            {f.servicio.nombre} · {f.cargo.nombre} ({f.metricas.length} métrica(s))
          </option>
        ))}
      </select>
      {sel && <MetricasEditor key={sel} servicioCargoId={sel} esRrhh={esRrhh} />}
    </section>
  )
}

export function MatrizPage() {
  const { data: me } = useMe()
  const esRrhh = me?.rol === 'rrhh_admin'
  const [params, setParams] = useSearchParams()
  // "Ir a corregir" del tablero llega con ?servicio=<id>: va a la composición salarial.
  const seccion = (params.get('seccion') as Seccion | null) ?? 'composicion'

  // Contadores de las pestañas (las listas se reutilizan desde caché en cada sección).
  const n = {
    clientes: useConfigLista<ClienteCfg>('clientes').data?.length,
    centros: useConfigLista<CentroCostoCfg>('centros-costo').data?.length,
    servicios: useConfigLista<ServicioCfg>('servicios').data?.length,
    cargos: useConfigLista<CargoCfg>('cargos').data?.length,
    composicion: useMatrizServicio().data?.length,
    contratos: useConfigLista<ContratoCfg>('contratos').data?.length,
    metricas: useMatrizServicio().data?.reduce((s, f) => s + f.metricas.length, 0),
    'catalogo-metricas': useConfigLista<MetricaCatalogoCfg>('catalogo-metricas').data?.length,
    'catalogo-bonos': useConfigLista<BonoCatalogoCfg>('catalogo-bonos').data?.length,
    pendientes: undefined,
  } satisfies Record<Seccion, number | undefined>

  const SECCIONES: [Seccion, string][] = [
    ['clientes', 'Clientes y proyectos'],
    ['centros', 'Centros de costo'],
    ['servicios', 'Servicios'],
    ['cargos', 'Cargos'],
    ['composicion', 'Composición salarial'],
    ['contratos', 'Contratos'],
    ['metricas', 'Métricas y bonos por cargo'],
    ['catalogo-metricas', 'Catálogo de métricas'],
    ['catalogo-bonos', 'Catálogo de bonos'],
    ['pendientes', 'Pendientes de completar'],
  ]

  const ir = (s: Seccion) => setParams({ seccion: s }, { replace: true })

  return (
    <div className="grid gap-5">
      <div>
        <h2 className="text-2xl font-semibold text-nova-titulo">Matriz de servicio</h2>
        <p className="mt-1 text-sm text-nova-gris">Clientes, servicios, cargos, sueldos, métricas y bonos.</p>
      </div>

      <nav aria-label="Secciones de la matriz" className="grid gap-1.5 rounded-xl border bg-card p-3 shadow-nova-xs sm:grid-cols-2 xl:grid-cols-5">
        {SECCIONES.map(([clave, titulo], i) => (
          <button
            key={clave}
            type="button"
            aria-current={seccion === clave ? 'page' : undefined}
            onClick={() => ir(clave)}
            className={cn(
              'flex items-center justify-between gap-2 rounded-lg px-3 py-2 text-left text-sm transition-colors',
              seccion === clave ? 'bg-nova-azul font-medium text-white' : 'text-nova-texto-2 hover:bg-nova-hielo',
            )}
          >
            <span>
              {i + 1}. {titulo}
            </span>
            {n[clave] !== undefined && (
              <span className={cn('rounded-full px-1.5 text-xs tabular-nums', seccion === clave ? 'bg-white/20' : 'bg-nova-hielo text-nova-gris')}>
                {n[clave]}
              </span>
            )}
          </button>
        ))}
      </nav>

      {seccion === 'clientes' && <ClientesYProyectos esRrhh={esRrhh} />}
      {seccion === 'centros' && <CentrosCosto esRrhh={esRrhh} />}
      {seccion === 'servicios' && <Servicios esRrhh={esRrhh} />}
      {seccion === 'cargos' && <CargosSeccion esRrhh={esRrhh} />}
      {seccion === 'composicion' && <ComposicionSalarial esRrhh={esRrhh} />}
      {seccion === 'contratos' && <Contratos esRrhh={esRrhh} />}
      {seccion === 'metricas' && <MetricasPorCargo esRrhh={esRrhh} />}
      {seccion === 'catalogo-metricas' && <CatalogoMetricas esRrhh={esRrhh} />}
      {seccion === 'catalogo-bonos' && <CatalogoBonos esRrhh={esRrhh} />}
      {seccion === 'pendientes' && <PendientesCompletar />}
    </div>
  )
}

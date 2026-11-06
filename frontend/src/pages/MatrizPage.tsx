import { useSearchParams } from 'react-router-dom'

import {
  type CargoCfg,
  type CentroCostoCfg,
  type ClienteCfg,
  type ContratoCfg,
  type ServicioCfg,
  useConfigLista,
  useMatrizServicio,
  useMe,
} from '@/api'
import { ComposicionSalarial } from '@/components/matriz/ComposicionSalarial'
import { CargosSeccion, CentrosCosto, ClientesYProyectos, Contratos, Servicios } from '@/components/matriz/Secciones'
import { cn } from '@/lib/utils'

type Seccion = 'clientes' | 'centros' | 'servicios' | 'cargos' | 'composicion' | 'contratos'

const SECCIONES: [Seccion, string][] = [
  // Orden acordado en la revisión del 04/11/2026. Proyecto se oculta por ahora (sigue en la base).
  ['clientes', 'Clientes'],
  ['centros', 'Centros de costo'],
  ['contratos', 'Contratos'],
  ['cargos', 'Cargos'],
  ['servicios', 'Servicios'],
  ['composicion', 'Composición salarial'],
]

export function MatrizPage() {
  const { data: me } = useMe()
  const esRrhh = me?.rol === 'rrhh_admin'
  const [params, setParams] = useSearchParams()
  // Por defecto (y para "Ir a corregir" del tablero, que llega con ?servicio=<id>): composición.
  const pedida = params.get('seccion')
  const editar = params.get('editar')
  const seccion: Seccion = SECCIONES.some(([s]) => s === pedida) ? (pedida as Seccion) : 'composicion'

  // Contadores de las pestañas (las listas se reutilizan desde caché en cada sección).
  const n: Record<Seccion, number | undefined> = {
    clientes: useConfigLista<ClienteCfg>('clientes').data?.length,
    centros: useConfigLista<CentroCostoCfg>('centros-costo').data?.length,
    servicios: useConfigLista<ServicioCfg>('servicios').data?.length,
    cargos: useConfigLista<CargoCfg>('cargos').data?.length,
    composicion: useMatrizServicio().data?.length,
    contratos: useConfigLista<ContratoCfg>('contratos').data?.length,
  }

  return (
    <div className="grid gap-5">
      <div>
        <h2 className="text-2xl font-semibold text-nova-titulo">Matriz de servicio</h2>
        <p className="mt-1 text-sm text-nova-gris">Clientes, servicios, cargos, sueldos, métricas y bonos.</p>
      </div>

      <div role="tablist" aria-label="Secciones de la matriz" className="-mx-1 flex gap-1 overflow-x-auto border-b px-1">
        {SECCIONES.map(([clave, titulo]) => (
          <button
            key={clave}
            role="tab"
            type="button"
            aria-selected={seccion === clave}
            onClick={() => setParams({ seccion: clave }, { replace: true })}
            className={cn(
              '-mb-px flex shrink-0 items-center gap-1.5 border-b-2 px-3 py-2 text-sm whitespace-nowrap transition-colors',
              seccion === clave
                ? 'border-nova-azul font-medium text-nova-azul'
                : 'border-transparent text-nova-texto-2 hover:border-border hover:text-nova-titulo',
            )}
          >
            {titulo}
            {n[clave] !== undefined && (
              <span className={cn('rounded-full px-1.5 text-[11px] leading-4 tabular-nums', seccion === clave ? 'bg-nova-hielo text-nova-azul' : 'bg-muted text-nova-gris')}>
                {n[clave]}
              </span>
            )}
          </button>
        ))}
      </div>

      <div role="tabpanel">
        {seccion === 'clientes' && <ClientesYProyectos esRrhh={esRrhh} />}
        {seccion === 'centros' && <CentrosCosto esRrhh={esRrhh} />}
        {seccion === 'contratos' && <Contratos esRrhh={esRrhh} abrirId={editar} />}
        {seccion === 'cargos' && <CargosSeccion esRrhh={esRrhh} />}
        {seccion === 'servicios' && <Servicios esRrhh={esRrhh} abrirId={editar} />}
        {seccion === 'composicion' && <ComposicionSalarial esRrhh={esRrhh} />}
      </div>
    </div>
  )
}

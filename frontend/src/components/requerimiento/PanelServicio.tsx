import { AlertTriangle, ChevronDown, Lock } from 'lucide-react'
import { type ReactNode, useState } from 'react'

import { type ServicioCfg, useConfigLista, useMatrizServicio, useParametros } from '@/api'
import { SolicitarCambio } from '@/components/solicitudes/SolicitarCambio'
import { Skeleton } from '@/components/ui/skeleton'
import { formatMonto } from '@/lib/format'
import { SALARIO_MINIMO_NACIONAL, formatParametro } from '@/lib/parametros'
import { cn } from '@/lib/utils'

const bs = (m: string | null | undefined) => (m == null ? '—' : `Bs ${formatMonto(m)}`)

/** Un dato del servicio con su botón para pedir el cambio (fuera del panel: conserva su estado). */
function Dato({
  titulo,
  campo,
  servicioId,
  cargos,
  cargoInicial,
  habilitado,
  children,
}: {
  titulo: string
  campo: string
  servicioId: string
  cargos: { id: string; nombre: string }[]
  /** Para sueldo y bonos: el cargo elegido (la base lo exige). */
  cargoInicial?: string
  habilitado: boolean
  children: ReactNode
}) {
  return (
    <div className="grid gap-1">
      <p className="text-xs text-nova-gris">{titulo}</p>
      <div className="text-sm font-medium text-nova-titulo">{children}</div>
      {habilitado && (
        <div className="[&>span]:ml-0 [&_button]:h-7 [&_button]:border-0 [&_button]:px-0 [&_button]:text-xs [&_button]:shadow-none [&_button]:hover:bg-transparent [&_button]:hover:text-nova-azul">
          <SolicitarCambio servicioId={servicioId} cargos={cargos} cargoInicial={cargoInicial} campoFijo={campo} />
        </div>
      )}
    </div>
  )
}

/**
 * Datos fijos del servicio elegido (no se editan en el requerimiento). Si alguno está mal, cada
 * uno tiene "Solicitar cambio": crea una solicitud con ese `campo` (valores del CHECK
 * `solicitudes_cambio_campo_check`), que resuelve el área responsable desde el Tablero.
 */
export function PanelServicio({
  servicioId,
  cargoId,
  sinServicio,
}: {
  servicioId: string
  cargoId: string
  /** Cargo administrativo sin servicio: no hay datos de servicio. */
  sinServicio: boolean
}) {
  const [abierto, setAbierto] = useState(true)
  const elegido = !!servicioId && !sinServicio
  const servicios = useConfigLista<ServicioCfg>('servicios')
  const matriz = useMatrizServicio({ servicio_id: servicioId }, { enabled: elegido })
  const parametros = useParametros()

  const s = elegido ? servicios.data?.find((x) => x.id === servicioId) : undefined
  const filas = elegido ? (matriz.data ?? []).filter((f) => f.servicio.id === servicioId) : []
  const fila = filas.find((f) => f.cargo.id === cargoId)
  const cargos = filas.map((f) => ({ id: f.cargo.id, nombre: f.cargo.nombre }))
  const smn = parametros.data?.vigentes.find((v) => v.parametro === SALARIO_MINIMO_NACIONAL)
  // TODO(negocio): confirmar contra qué se compara el SMN (hoy: haber básico diurno, como el
  // "Sueldo básico" del original; ¿o diurno + nocturno?).
  const bajoMinimo = !!fila && !!smn && Number(fila.haber_basico_diurno) < Number(smn.valor)
  const cargando = elegido && (servicios.isPending || matriz.isPending)

  const comun = { servicioId, cargos, habilitado: !!s }

  return (
    <aside className="rounded-xl border bg-card shadow-nova-sm lg:sticky lg:top-20" aria-label="Datos del servicio">
      <button
        type="button"
        onClick={() => setAbierto(!abierto)}
        aria-expanded={abierto}
        className="flex w-full items-center gap-2 px-5 py-4 text-left font-semibold text-nova-titulo"
      >
        <Lock className="size-4 text-nova-azul" aria-hidden />
        Datos del servicio
        <ChevronDown className={cn('ml-auto size-4 transition-transform', abierto && 'rotate-180')} aria-hidden />
      </button>
      {abierto && (
        <div className="grid gap-4 border-t px-5 py-4">
          {sinServicio ? (
            <p className="text-sm text-nova-gris">Cargo administrativo sin servicio: no hay datos de servicio.</p>
          ) : cargando ? (
            <Skeleton className="h-64 rounded-lg" />
          ) : (
            <>
              {!elegido && <p className="text-xs text-nova-gris">Elegí servicio y cargo para ver sus datos.</p>}
              <Dato titulo="Centro de costo" campo="centro_costo" {...comun}>{s?.centro_costo_nombre ?? '—'}</Dato>
              <Dato titulo="Cliente" campo="cliente" {...comun}>{s?.cliente_nombre ?? '—'}</Dato>
              <Dato titulo="Contrato comercial" campo="contrato_comercial" {...comun}>{s?.contrato_nombre ?? '—'}</Dato>
              <Dato titulo="Tipo de contrato" campo="tipo_contrato" {...comun}>{s?.tipo_contrato ?? '—'}</Dato>
              <Dato titulo="Modalidad" campo="modalidad" {...comun}>
                {s?.modalidad ? s.modalidad.charAt(0).toUpperCase() + s.modalidad.slice(1) : '—'}
              </Dato>
              <Dato titulo="Días de trabajo" campo="dias_trabajo" {...comun}>
                {s?.dias_trabajo.length ? (
                  <ul className="flex flex-wrap gap-1">
                    {s.dias_trabajo.map((d) => (
                      <li key={d} className="rounded-full bg-nova-hielo px-2 py-0.5 text-xs font-normal text-nova-azul">
                        {d}
                      </li>
                    ))}
                  </ul>
                ) : (
                  '—'
                )}
              </Dato>

              <hr />

              <Dato titulo="Sueldo básico" campo="sueldo_basico" {...comun} cargoInicial={cargoId} habilitado={!!s && !!fila}>
                {fila ? (
                  <div className="grid gap-1">
                    <p className="text-xl font-semibold tabular-nums text-nova-azul">{bs(fila.haber_basico_diurno)}</p>
                    {fila.haber_basico_nocturno != null && (
                      <p className="text-xs font-normal text-nova-texto-2">Nocturno: {bs(fila.haber_basico_nocturno)}</p>
                    )}
                    {bajoMinimo && smn && (
                      <p className="flex gap-1.5 rounded-lg bg-advertencia/15 p-2 text-xs font-normal text-[#B45309]" role="alert">
                        <AlertTriangle className="size-3.5 shrink-0" aria-hidden />
                        Por debajo del Salario Mínimo Nacional vigente ({formatParametro(smn.parametro, smn.valor)})
                      </p>
                    )}
                  </div>
                ) : elegido && cargoId ? (
                  <span className="font-normal text-nova-gris">Sin composición visible para este cargo.</span>
                ) : (
                  '—'
                )}
              </Dato>
              <Dato titulo="Bonos fijos" campo="bonos" {...comun} cargoInicial={cargoId} habilitado={!!s && !!fila}>
                {!fila ? (
                  '—'
                ) : fila.metricas.length === 0 ? (
                  <span className="font-normal text-nova-gris">Sin bonos configurados.</span>
                ) : (
                  <ul className="grid gap-0.5 font-normal">
                    {fila.metricas.map((m) => (
                      <li key={m.id} className="flex justify-between gap-3">
                        <span>{m.nombre}</span>
                        <span className="tabular-nums">{bs(m.monto_bono)}</span>
                      </li>
                    ))}
                  </ul>
                )}
              </Dato>
              {fila && (
                <p className="text-xs text-nova-gris">
                  Total ganado: <span className="font-semibold text-nova-azul">{bs(fila.total_ganado)}</span> (sin ajustar por
                  cumplimiento real). Se congela al crear el requerimiento.
                </p>
              )}
            </>
          )}
        </div>
      )}
    </aside>
  )
}

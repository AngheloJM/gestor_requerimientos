import { ArrowLeft, Lock } from 'lucide-react'
import type { ReactNode } from 'react'
import { Link, useParams } from 'react-router-dom'

import { type BonoCongelado, useRequerimiento } from '@/api'
import { EstadoRequerimiento, etiquetaTipo } from '@/components/EstadoRequerimiento'
import { ErrorApi } from '@/components/ErrorApi'
import { EntregasParciales } from '@/components/requerimiento/EntregasParciales'
import { HistorialCambios } from '@/components/requerimiento/HistorialCambios'
import { PanelSolicitante } from '@/components/requerimiento/PanelSolicitante'
import { Skeleton } from '@/components/ui/skeleton'
import { formatFecha, formatMonto } from '@/lib/format'

const TIPO_CARGO: Record<string, string> = { operativo: 'Operativo', administrativo_staff: 'Administrativo (staff)' }
const TURNO: Record<string, string> = { continuo: 'Continuo', turno: 'Por turno' }

const vacio = (v: string | number | null | undefined) => (v == null || v === '' ? '—' : v)
const bs = (m: string | number | null | undefined) => (m == null ? '—' : `Bs ${formatMonto(String(m))}`)

function Seccion({ titulo, children }: { titulo: string; children: ReactNode }) {
  return (
    <section className="rounded-2xl border bg-card p-6 shadow-nova-sm">
      <h3 className="mb-5 text-base font-semibold text-nova-titulo">{titulo}</h3>
      {children}
    </section>
  )
}

function Dato({ etiqueta, children }: { etiqueta: string; children: ReactNode }) {
  return (
    <div className="min-w-0">
      <dt className="text-xs text-nova-gris">{etiqueta}</dt>
      <dd className="mt-1 text-sm break-words text-nova-titulo">{children}</dd>
    </div>
  )
}

function DatoFijo({ etiqueta, children }: { etiqueta: string; children: ReactNode }) {
  return (
    <div>
      <dt className="text-xs text-nova-gris">{etiqueta}</dt>
      <dd className="mt-0.5 text-sm break-words text-nova-titulo">{children}</dd>
    </div>
  )
}

const Grilla = ({ children }: { children: ReactNode }) => <dl className="grid gap-x-6 gap-y-5 sm:grid-cols-2 lg:grid-cols-3">{children}</dl>

function Bonos({ bonos }: { bonos: BonoCongelado[] | null }) {
  if (!bonos || bonos.length === 0) return <p className="text-sm text-nova-gris">Sin bonos.</p>
  return (
    <ul className="divide-y rounded-lg border">
      {bonos.map((b, i) => (
        <li key={i} className="flex flex-wrap items-start justify-between gap-2 px-3 py-2 text-sm">
          <div>
            <p className="text-nova-titulo">{b.metrica ?? b.nombre ?? '—'}</p>
            {b.tipo === 'extra' && <p className="text-xs text-nova-gris">Bono extra{b.motivo ? ` · ${b.motivo}` : ''}</p>}
          </div>
          <span className="tabular-nums text-nova-titulo">{bs(b.monto)}</span>
        </li>
      ))}
    </ul>
  )
}

/** Detalle de solo lectura de un requerimiento (lo que RLS deja ver al usuario). */
export function RequerimientoDetallePage() {
  const { id } = useParams()
  const { data: r, isPending, error } = useRequerimiento(id)

  return (
    <div className="grid gap-6">
      <Link to="/requerimientos" className="inline-flex w-fit items-center gap-2 text-sm font-medium text-nova-titulo hover:text-nova-azul">
        <ArrowLeft className="size-4" aria-hidden /> Volver a Requerimientos
      </Link>

      {error ? (
        <ErrorApi error={error} />
      ) : isPending ? (
        <Skeleton className="h-96 rounded-2xl" />
      ) : (
        <>
          <div className="flex flex-wrap items-end justify-between gap-3">
            <div>
              <p className="text-xs text-nova-gris">Detalle del requerimiento</p>
              <h2 className="text-2xl font-semibold text-nova-titulo">{r.folio ?? `#${r.numero}`}</h2>
            </div>
            <EstadoRequerimiento valor={r.estado} />
          </div>

          <div className="grid items-start gap-5 lg:grid-cols-[minmax(0,1fr)_21rem]">
            <div className="grid min-w-0 gap-5">
              <Seccion titulo="1. Tipo de requerimiento">
                <Grilla>
                  <Dato etiqueta="Tipo">{etiquetaTipo(r.tipo)}</Dato>
                  <Dato etiqueta="Número">{r.numero}</Dato>
                  <Dato etiqueta="Folio">{vacio(r.folio)}</Dato>
                  <Dato etiqueta="Cantidad de personas requeridas">{vacio(r.cantidad)}</Dato>
                  <Dato etiqueta="Motivo del requerimiento">{vacio(r.motivo)}</Dato>
                  <Dato etiqueta="Creado">{formatFecha(r.creado.slice(0, 10))}</Dato>
                </Grilla>
              </Seccion>

              <Seccion titulo="2. Entregas parciales">
                <EntregasParciales r={r} />
              </Seccion>

              <Seccion titulo="3. Servicio y cargo">
                <Grilla>
                  <Dato etiqueta="Servicio">{r.servicio_nombre ?? 'Sin servicio (staff)'}</Dato>
                  <Dato etiqueta="Código de servicio">{vacio(r.servicio_codigo)}</Dato>
                  <Dato etiqueta="Cargo">{vacio(r.cargo_nombre)}</Dato>
                  <Dato etiqueta="Tipo de cargo">{r.cargo_tipo ? (TIPO_CARGO[r.cargo_tipo] ?? r.cargo_tipo) : '—'}</Dato>
                </Grilla>
              </Seccion>

              <Seccion titulo="4. Fecha">
                <Grilla>
                  <Dato etiqueta="Fecha de inicio de operación">{formatFecha(r.fecha_inicio_operacion)}</Dato>
                </Grilla>
              </Seccion>

              <Seccion titulo="5. Sueldo y bonos congelados al crear">
                <div className="grid gap-5">
                  <Grilla>
                    <Dato etiqueta="Haber básico diurno">{bs(r.haber_basico_diurno)}</Dato>
                    <Dato etiqueta="Haber básico nocturno">{bs(r.haber_basico_nocturno)}</Dato>
                  </Grilla>
                  <div>
                    <p className="mb-2 text-xs text-nova-gris">Bonos</p>
                    <Bonos bonos={r.bonos} />
                  </div>
                </div>
              </Seccion>

              <Seccion titulo="6. Comentarios adicionales">
                <p className="text-sm whitespace-pre-wrap text-nova-titulo">{r.comentarios ?? <span className="text-nova-gris">Sin comentarios.</span>}</p>
              </Seccion>

              <Seccion titulo="7. Historial de cambios">
                <HistorialCambios id={r.id} />
              </Seccion>
            </div>

            {/* Datos fijos, a la derecha (como en "Nuevo requerimiento"). */}
            <div className="grid min-w-0 gap-5 lg:sticky lg:top-20">
              <PanelSolicitante
                datos={{
                  nombre: r.solicitante_nombre,
                  cargo: r.solicitante_cargo,
                  division: r.solicitante_division,
                  region: r.solicitante_region,
                  inmediato_superior: r.inmediato_superior,
                }}
              />
              <aside className="rounded-xl border bg-card shadow-nova-sm" aria-label="Datos del servicio">
                <div className="flex items-center gap-2 px-5 py-4">
                  <Lock className="size-4 text-nova-azul" aria-hidden />
                  <h3 className="font-semibold text-nova-titulo">Datos del servicio</h3>
                </div>
                <dl className="grid gap-3 border-t px-5 py-4">
                  <DatoFijo etiqueta="Centro de costo">{vacio(r.centro_costo)}</DatoFijo>
                  <DatoFijo etiqueta="Cliente">{vacio(r.cliente)}</DatoFijo>
                  <DatoFijo etiqueta="Contrato comercial">{vacio(r.contrato)}</DatoFijo>
                  <DatoFijo etiqueta="Tipo de contrato">{vacio(r.tipo_contrato)}</DatoFijo>
                  <DatoFijo etiqueta="Modalidad">{vacio(r.modalidad)}</DatoFijo>
                  <DatoFijo etiqueta="Tipo de turno">{r.tipo_turno ? (TURNO[r.tipo_turno] ?? r.tipo_turno) : '—'}</DatoFijo>
                  <DatoFijo etiqueta="Días de trabajo">{r.dias_trabajo?.length ? r.dias_trabajo.join(', ') : '—'}</DatoFijo>
                  <DatoFijo etiqueta="Departamento / región">{vacio(r.departamento_region)}</DatoFijo>
                </dl>
              </aside>
            </div>
          </div>
        </>
      )}
    </div>
  )
}

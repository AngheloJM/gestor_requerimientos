import { ArrowUpRight, Check, Undo2, X } from 'lucide-react'
import { useState } from 'react'
import { Link } from 'react-router-dom'

import { type TarjetaKanban, useCerrarSolicitud, useMe } from '@/api'
import { EstadoBadge, type Tono } from '@/components/EstadoBadge'
import { ErrorApi } from '@/components/ErrorApi'
import { Button } from '@/components/ui/button'
import { formatFecha } from '@/lib/format'
import { CAMPOS_SOLICITUD, ESTADOS_SOLICITUD, accionesPermitidas } from '@/lib/solicitudes'
import { cn } from '@/lib/utils'

const TONO: Record<string, Tono> = { pendiente: 'advertencia', resuelta: 'exito', rechazada: 'error', cancelada: 'neutral' }

type Accion = 'resolver' | 'rechazar' | 'cancelar'

/** Tarjeta del Kanban con sus acciones (ver reglas en `accionesPermitidas`). */
export function TarjetaSolicitud({ t }: { t: TarjetaKanban }) {
  const { data: me } = useMe()
  const cerrar = useCerrarSolicitud()
  const [accion, setAccion] = useState<Accion | null>(null)
  const [comentario, setComentario] = useState('')
  const [error, setError] = useState<string>()
  const permisos = accionesPermitidas(t, me)
  const cerrada = t.estado !== 'pendiente'

  const confirmar = () => {
    if (!accion) return
    if (accion !== 'cancelar' && comentario.trim().length < 3) {
      setError('El comentario es obligatorio (al menos 3 caracteres).')
      return
    }
    setError(undefined)
    cerrar.mutate({ id: t.id, accion, comentario: comentario.trim() }, { onSuccess: () => setAccion(null) })
  }

  return (
    <li className={cn('rounded-lg border bg-white p-3 shadow-nova-xs transition-shadow hover:shadow-nova-md', cerrada && 'opacity-70')}>
      <div className="flex items-start justify-between gap-2">
        <p className="font-medium text-nova-titulo">{t.servicio.nombre ?? 'Servicio'}</p>
        {t.estado && <EstadoBadge tono={TONO[t.estado] ?? 'neutral'}>{ESTADOS_SOLICITUD[t.estado] ?? t.estado}</EstadoBadge>}
      </div>
      <p className="mt-1 text-sm text-nova-texto-2">
        Cambio en <span className="font-medium">{CAMPOS_SOLICITUD[t.campo] ?? t.campo}</span>
        {t.cargo?.nombre ? ` · ${t.cargo.nombre}` : ''}
      </p>
      {t.motivo && <p className="mt-2 line-clamp-2 text-sm text-nova-gris">{t.motivo}</p>}
      {t.comentario_resolucion && <p className="mt-2 rounded bg-nova-hielo/60 p-2 text-xs text-nova-texto-2">“{t.comentario_resolucion}”</p>}
      <p className="mt-2 text-xs text-nova-gris">{formatFecha(t.created_at.slice(0, 10))}</p>

      {!cerrada && (
        <div className="mt-3 flex flex-wrap gap-1.5">
          <Link to={`/matriz?servicio=${encodeURIComponent(t.servicio.id)}`} className="inline-flex items-center gap-1 rounded-md border px-2 py-1 text-xs hover:bg-nova-hielo">
            <ArrowUpRight className="size-3.5" aria-hidden /> Ir a corregir
          </Link>
          {permisos.cerrar && (
            <>
              <button type="button" onClick={() => setAccion('resolver')} className="inline-flex items-center gap-1 rounded-md border px-2 py-1 text-xs text-exito hover:bg-exito/10">
                <Check className="size-3.5" aria-hidden /> Resolver
              </button>
              <button type="button" onClick={() => setAccion('rechazar')} className="inline-flex items-center gap-1 rounded-md border px-2 py-1 text-xs text-error hover:bg-error/10">
                <X className="size-3.5" aria-hidden /> Rechazar
              </button>
            </>
          )}
          {permisos.cancelar && (
            <button type="button" onClick={() => setAccion('cancelar')} className="inline-flex items-center gap-1 rounded-md border px-2 py-1 text-xs text-nova-texto-2 hover:bg-muted">
              <Undo2 className="size-3.5" aria-hidden /> Cancelar solicitud
            </button>
          )}
        </div>
      )}

      {accion && (
        <div className="mt-3 grid gap-2 rounded-lg border bg-nova-hielo/40 p-2.5">
          <label htmlFor={`com-${t.id}`} className="text-xs font-medium text-nova-titulo">
            {accion === 'cancelar' ? 'Comentario (opcional)' : 'Comentario (obligatorio)'}
          </label>
          <textarea
            id={`com-${t.id}`}
            rows={2}
            maxLength={2000}
            value={comentario}
            onChange={(e) => setComentario(e.target.value)}
            className="w-full rounded-md border border-input bg-white px-2 py-1.5 text-sm"
          />
          {error && (
            <p className="text-xs text-error" role="alert">
              {error}
            </p>
          )}
          {cerrar.error && <ErrorApi error={cerrar.error} />}
          <div className="flex justify-end gap-1.5">
            <Button type="button" size="sm" variant="ghost" onClick={() => setAccion(null)}>
              Volver
            </Button>
            <Button type="button" size="sm" disabled={cerrar.isPending} onClick={confirmar}>
              {accion === 'resolver' ? 'Confirmar resolución' : accion === 'rechazar' ? 'Confirmar rechazo' : 'Cancelar solicitud'}
            </Button>
          </div>
        </div>
      )}
    </li>
  )
}

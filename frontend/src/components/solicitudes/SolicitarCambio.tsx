import { CheckCircle2, MessageSquarePlus } from 'lucide-react'
import { useState } from 'react'

import { useCrearSolicitud } from '@/api'
import { ErrorApi } from '@/components/ErrorApi'
import { Button } from '@/components/ui/button'
import { CAMPOS_CON_CARGO, CAMPOS_SOLICITUD } from '@/lib/solicitudes'

/**
 * Formulario compacto para pedir un cambio sobre un servicio (lo resuelve el área responsable).
 * Puede usarse solo (maneja su estado) o controlado desde afuera (`abierto`/`onAbrir`/`onCerrar`),
 * p. ej. el botón en una celda y el formulario en una fila extendida (`soloBoton`).
 */
export function SolicitarCambio({
  servicioId,
  cargos,
  cargoInicial,
  abierto: abiertoExterno,
  onAbrir,
  onCerrar,
  soloBoton,
  onEnviada,
  campoFijo,
}: {
  servicioId: string
  cargos: { id: string; nombre: string }[]
  cargoInicial?: string
  abierto?: boolean
  onAbrir?: () => void
  onCerrar?: () => void
  soloBoton?: boolean
  /** Se llama al crear la solicitud (útil en modo controlado). */
  onEnviada?: () => void
  /**
   * Dato a cambiar ya decidido (ej. desde un dato del servicio). Oculta el selector de campo y,
   * si el campo pide cargo y viene `cargoInicial`, también el de cargo. Formulario en una columna.
   */
  campoFijo?: string
}) {
  const crear = useCrearSolicitud()
  const [abiertoInterno, setAbiertoInterno] = useState(false)
  const abierto = abiertoExterno ?? abiertoInterno
  const setAbierto = (v: boolean) => {
    if (abiertoExterno === undefined) setAbiertoInterno(v)
    if (v) onAbrir?.()
    else onCerrar?.()
  }
  const [campo, setCampo] = useState(campoFijo ?? '')
  const [cargo, setCargo] = useState(cargoInicial ?? '')
  const [motivo, setMotivo] = useState('')
  const [descripcion, setDescripcion] = useState('')
  const [error, setError] = useState<string>()
  const [ok, setOk] = useState(false)
  const pideCargo = CAMPOS_CON_CARGO.includes(campo)

  const enviar = (e: React.FormEvent) => {
    e.preventDefault()
    if (!campo) return setError('Elegí qué dato hay que cambiar.')
    if (pideCargo && !cargo) return setError('Elegí el cargo.')
    if (motivo.trim().length < 3) return setError('El motivo debe tener al menos 3 caracteres.')
    setError(undefined)
    crear.mutate(
      { servicio_id: servicioId, cargo_id: pideCargo ? cargo : null, campo, motivo: motivo.trim(), descripcion: descripcion.trim() || null },
      {
        onSuccess: () => {
          setOk(true)
          onEnviada?.()
          setAbierto(false)
          setCampo(campoFijo ?? '')
          setCargo(cargoInicial ?? '')
          setMotivo('')
          setDescripcion('')
        },
      },
    )
  }

  if (!abierto || soloBoton) {
    return (
      <span className="ml-auto flex items-center gap-2">
        {ok && (
          <span className="flex items-center gap-1 text-xs text-exito" role="status">
            <CheckCircle2 className="size-3.5" aria-hidden /> Solicitud enviada
          </span>
        )}
        <Button
          type="button"
          size="sm"
          variant="outline"
          aria-expanded={abierto}
          onClick={() => {
            setAbierto(!abierto)
            setOk(false)
          }}
        >
          <MessageSquarePlus aria-hidden /> Solicitar cambio
        </Button>
      </span>
    )
  }

  const campoClase = 'h-8 rounded-md border border-input bg-white px-2 text-sm'
  return (
    <form
      onSubmit={enviar}
      className={campoFijo ? 'mt-2 grid w-full gap-2 rounded-lg border bg-white p-3 [&>*]:col-span-1' : 'mt-2 grid w-full gap-2 rounded-lg border bg-white p-3 sm:grid-cols-2'}
      aria-label="Solicitar cambio"
    >
      {campoFijo ? (
        <p className="text-xs text-nova-gris">
          Cambio en <span className="font-medium text-nova-titulo">{CAMPOS_SOLICITUD[campoFijo] ?? campoFijo}</span>
          {pideCargo && cargoInicial && <> · {cargos.find((c) => c.id === cargoInicial)?.nombre}</>}
        </p>
      ) : (
        <select aria-label="Dato a cambiar" className={campoClase} value={campo} onChange={(e) => setCampo(e.target.value)}>
          <option value="">¿Qué dato hay que cambiar?</option>
          {Object.entries(CAMPOS_SOLICITUD).map(([v, t]) => (
            <option key={v} value={v}>
              {t}
            </option>
          ))}
        </select>
      )}
      {campoFijo && (!pideCargo || cargoInicial) ? null : pideCargo ? (
        <select aria-label="Cargo" className={campoClase} value={cargo} onChange={(e) => setCargo(e.target.value)}>
          <option value="">Cargo…</option>
          {cargos.map((c) => (
            <option key={c.id} value={c.id}>
              {c.nombre}
            </option>
          ))}
        </select>
      ) : (
        <span />
      )}
      <input aria-label="Motivo" className={`${campoClase} sm:col-span-2`} maxLength={500} placeholder="Motivo (obligatorio)" value={motivo} onChange={(e) => setMotivo(e.target.value)} />
      <textarea
        aria-label="Descripción"
        rows={2}
        maxLength={2000}
        placeholder="Detalle (opcional)"
        className="rounded-md border border-input bg-white px-2 py-1.5 text-sm sm:col-span-2"
        value={descripcion}
        onChange={(e) => setDescripcion(e.target.value)}
      />
      {error && (
        <p className="text-xs text-error sm:col-span-2" role="alert">
          {error}
        </p>
      )}
      {crear.error && (
        <div className="sm:col-span-2">
          <ErrorApi error={crear.error} />
        </div>
      )}
      <div className="flex justify-end gap-1.5 sm:col-span-2">
        <Button type="button" size="sm" variant="ghost" onClick={() => setAbierto(false)}>
          Cerrar
        </Button>
        <Button type="submit" size="sm" disabled={crear.isPending}>
          Enviar solicitud
        </Button>
      </div>
    </form>
  )
}

import { Info, Pencil, Plus, Trash2, X } from 'lucide-react'
import { type ReactNode, useState } from 'react'

import { type RecursoConfig, useConfigEliminar, useConfigGuardar } from '@/api'
import { ErrorApi } from '@/components/ErrorApi'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Switch } from '@/components/ui/switch'
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import { cn } from '@/lib/utils'

export type Valor = string | string[] | boolean
export type Valores = Record<string, Valor>

export interface Campo {
  nombre: string
  etiqueta: string
  tipo: 'texto' | 'textarea' | 'numero' | 'fecha' | 'select' | 'multiselect' | 'switch'
  /** Opciones [valor, texto]; pueden depender de otros campos (ej. proyecto según cliente). */
  opciones?: (v: Valores) => [string, string][]
  /** Solo `select`: agrega "Otro…" para escribir un valor fuera de la lista. */
  otro?: boolean
  requerido?: boolean
  ayuda?: string
  /** Deshabilita (y vacía al enviar, si corresponde) según otros campos. */
  deshabilitado?: (v: Valores) => boolean
  /** Ajusta otros campos al cambiar este (ej. limpiar proyecto al cambiar cliente). */
  alCambiar?: (v: Valores) => Valores
  ancho?: 'completo'
}

export interface Columna<T> {
  titulo: string
  valor: (t: T) => ReactNode
  clase?: string
}

const entrada =
  'h-9 w-full rounded-lg border border-input bg-white px-2.5 text-sm outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:bg-muted disabled:opacity-60'

const OTRO = '__otro__'

/** Desplegable; con `otro`, permite escribir un valor fuera de la lista. */
function Desplegable({
  c,
  v,
  set,
  id,
  deshabilitado,
}: {
  c: Campo
  v: Valores
  set: (nombre: string, valor: Valor) => void
  id: string
  deshabilitado: boolean
}) {
  const opciones = c.opciones?.(v) ?? []
  const valor = (v[c.nombre] as string) ?? ''
  const fueraDeLista = valor !== '' && !opciones.some(([o]) => o === valor)
  const [escribiendo, setEscribiendo] = useState(fueraDeLista)
  return (
    <div className="flex gap-2">
      <select
        id={id}
        className={entrada}
        disabled={deshabilitado}
        value={escribiendo ? OTRO : valor}
        onChange={(e) => {
          const nuevo = e.target.value
          setEscribiendo(nuevo === OTRO)
          set(c.nombre, nuevo === OTRO ? '' : nuevo)
        }}
      >
        {/* A confirmar con la clienta: redacción más formal ("Seleccione una opción"). */}
        <option value="">{c.requerido ? 'Seleccione una opción' : 'Sin asignar'}</option>
        {opciones.map(([val, txt]) => (
          <option key={val} value={val}>
            {txt}
          </option>
        ))}
        {c.otro && <option value={OTRO}>Otro…</option>}
      </select>
      {c.otro && escribiendo && (
        <Input
          aria-label={`${c.etiqueta} (otro)`}
          className="h-9 bg-white"
          autoFocus
          disabled={deshabilitado}
          value={valor}
          onChange={(e) => set(c.nombre, e.target.value)}
        />
      )}
    </div>
  )
}

function Control({ c, v, set }: { c: Campo; v: Valores; set: (nombre: string, valor: Valor) => void }) {
  const id = `campo-${c.nombre}`
  const deshabilitado = c.deshabilitado?.(v) ?? false
  const valor = v[c.nombre]
  switch (c.tipo) {
    case 'textarea':
      return (
        <textarea id={id} rows={3} className={cn(entrada, 'h-auto py-2')} disabled={deshabilitado} value={valor as string} onChange={(e) => set(c.nombre, e.target.value)} />
      )
    case 'select':
      return <Desplegable c={c} v={v} set={set} id={id} deshabilitado={deshabilitado} />
    case 'multiselect': {
      const sel = new Set(valor as string[])
      return (
        <div role="group" aria-labelledby={`${id}-et`} className="flex flex-wrap gap-1.5">
          {(c.opciones?.(v) ?? []).map(([val, txt]) => (
            <label
              key={val}
              className={cn(
                'cursor-pointer rounded-md border px-2.5 py-1 text-sm select-none',
                sel.has(val) ? 'border-nova-azul bg-nova-hielo text-nova-azul' : 'bg-white text-nova-texto-2',
                deshabilitado && 'pointer-events-none opacity-60',
              )}
            >
              <input
                type="checkbox"
                className="sr-only"
                checked={sel.has(val)}
                disabled={deshabilitado}
                onChange={() => {
                  const s = new Set(sel)
                  if (s.has(val)) s.delete(val)
                  else s.add(val)
                  set(c.nombre, [...s])
                }}
              />
              {txt}
            </label>
          ))}
        </div>
      )
    }
    case 'switch':
      return (
        <span className="flex h-9 items-center gap-2 text-sm">
          <Switch id={id} checked={valor as boolean} disabled={deshabilitado} onChange={(b) => set(c.nombre, b)} />
          <span className={valor ? 'font-medium text-nova-azul' : 'text-nova-gris'}>{valor ? 'Sí' : 'No'}</span>
        </span>
      )
    default:
      return (
        <Input
          id={id}
          type={c.tipo === 'fecha' ? 'date' : 'text'}
          inputMode={c.tipo === 'numero' ? 'decimal' : undefined}
          className="h-9 bg-white"
          disabled={deshabilitado}
          value={valor as string}
          onChange={(e) => set(c.nombre, e.target.value)}
        />
      )
  }
}

/** Formulario de alta/edición de una sección. */
function Formulario({
  titulo,
  campos,
  inicial,
  validar,
  enviar,
  cerrar,
  pendiente,
  error,
}: {
  titulo: string
  campos: Campo[]
  inicial: Valores
  validar?: (v: Valores) => string | undefined
  enviar: (v: Valores) => void
  cerrar: () => void
  pendiente: boolean
  error: unknown
}) {
  const [v, setV] = useState<Valores>(inicial)
  const [aviso, setAviso] = useState<string>()
  const set = (nombre: string, valor: Valor) => {
    const c = campos.find((x) => x.nombre === nombre)
    const nuevo = { ...v, [nombre]: valor }
    setV(c?.alCambiar ? c.alCambiar(nuevo) : nuevo)
  }
  const onSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    const faltante = campos.find((c) => c.requerido && !(c.deshabilitado?.(v) ?? false) && (v[c.nombre] === '' || v[c.nombre] === undefined))
    const msg = faltante ? `Completá “${faltante.etiqueta}”.` : validar?.(v)
    setAviso(msg)
    if (!msg) enviar(v)
  }
  return (
    <form onSubmit={onSubmit} className="grid gap-4 rounded-xl border border-nova-azul/30 bg-card p-5 shadow-nova-md" aria-label={titulo}>
      <div className="flex items-center justify-between">
        <h4 className="font-semibold text-nova-titulo">{titulo}</h4>
        <Button type="button" variant="ghost" size="icon" onClick={cerrar} aria-label="Cerrar formulario">
          <X />
        </Button>
      </div>
      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {campos.map((c) => (
          <div key={c.nombre} className={cn('grid content-start gap-1.5', (c.ancho === 'completo' || c.tipo === 'multiselect' || c.tipo === 'textarea') && 'sm:col-span-2 lg:col-span-3')}>
            <Label htmlFor={`campo-${c.nombre}`} id={`campo-${c.nombre}-et`}>
              {c.etiqueta}
              {c.requerido && <span className="text-error"> *</span>}
            </Label>
            <Control c={c} v={v} set={set} />
            {c.ayuda && <p className="text-xs text-nova-gris">{c.ayuda}</p>}
          </div>
        ))}
      </div>
      {aviso && (
        <p className="text-sm text-error" role="alert">
          {aviso}
        </p>
      )}
      {error ? <ErrorApi error={error} /> : null}
      <div className="flex justify-end gap-2">
        <Button type="button" variant="outline" onClick={cerrar}>
          Cancelar
        </Button>
        <Button type="submit" disabled={pendiente}>
          {pendiente ? 'Guardando…' : 'Guardar'}
        </Button>
      </div>
    </form>
  )
}

/**
 * Sección de configuración genérica: tabla + alta/edición/baja. La autorización real la aplica
 * la base; `permisos` solo decide qué botones mostrar.
 */
export function CrudSeccion<T>({
  titulo,
  tituloNuevo,
  descripcion,
  recurso,
  filas,
  cargando,
  error,
  columnas,
  campos,
  inicial,
  aBody,
  validar,
  idDe,
  nombreDe,
  permisos,
  nota,
  extraEdicion,
  abrirId,
}: {
  titulo: string
  /** Título del formulario de alta (por defecto "Nuevo · {titulo}"). */
  tituloNuevo?: string
  descripcion?: string
  recurso: RecursoConfig
  filas: T[] | undefined
  cargando: boolean
  error: unknown
  columnas: Columna<T>[]
  campos: Campo[]
  inicial: (t?: T) => Valores
  aBody: (v: Valores) => unknown
  validar?: (v: Valores) => string | undefined
  idDe: (t: T) => string
  nombreDe: (t: T) => string
  permisos: { crear: boolean; editar: boolean; eliminar: boolean }
  nota?: ReactNode
  /** Contenido adicional bajo el formulario al editar un registro existente. */
  extraEdicion?: (t: T) => ReactNode
  /** Registro a abrir en edición al cargar (p. ej. al llegar desde el tablero con ?editar=<id>). */
  abrirId?: string | null
}) {
  const guardar = useConfigGuardar(recurso)
  const eliminar = useConfigEliminar(recurso)
  const [editando, setEditando] = useState<T | 'nuevo' | null>(null)
  const [borrar, setBorrar] = useState<string | null>(null)
  const [hecho, setHecho] = useState<string>()

  const abrir = (t: T | 'nuevo') => {
    guardar.reset()
    setHecho(undefined)
    setEditando(t)
  }

  // Apertura pedida por la URL: se aplica una sola vez por id, apenas llegan las filas.
  const [abiertoDesdeUrl, setAbiertoDesdeUrl] = useState<string | null>(null)
  if (abrirId && permisos.editar && abiertoDesdeUrl !== abrirId) {
    const t = filas?.find((f) => idDe(f) === abrirId)
    if (t) {
      setAbiertoDesdeUrl(abrirId)
      setEditando(t)
    }
  }

  return (
    <section className="grid gap-4">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h3 className="text-lg font-semibold text-nova-titulo">{titulo}</h3>
          {descripcion && <p className="text-sm text-nova-gris">{descripcion}</p>}
        </div>
        {permisos.crear && (
          <Button onClick={() => abrir('nuevo')}>
            <Plus aria-hidden /> Nuevo
          </Button>
        )}
      </div>
      {nota && (
        <p className="flex gap-2 rounded-lg border border-info/30 bg-info/5 p-3 text-xs text-nova-texto-2">
          <Info className="size-3.5 shrink-0 text-info" aria-hidden />
          <span>{nota}</span>
        </p>
      )}
      {hecho && (
        <p className="text-sm text-exito" role="status">
          {hecho}
        </p>
      )}
      {editando && (
        <Formulario
          key={editando === 'nuevo' ? 'nuevo' : idDe(editando)}
          titulo={editando === 'nuevo' ? (tituloNuevo ?? `Nuevo · ${titulo}`) : `Editar · ${nombreDe(editando)}`}
          campos={campos}
          inicial={inicial(editando === 'nuevo' ? undefined : editando)}
          validar={validar}
          pendiente={guardar.isPending}
          error={guardar.error}
          cerrar={() => setEditando(null)}
          enviar={(v) =>
            guardar.mutate(
              { id: editando === 'nuevo' ? undefined : idDe(editando), body: aBody(v) },
              {
                onSuccess: () => {
                  setHecho(editando === 'nuevo' ? 'Creado.' : 'Guardado.')
                  setEditando(null)
                },
              },
            )
          }
        />
      )}
      {editando && editando !== 'nuevo' && extraEdicion?.(editando)}
      {eliminar.error ? <ErrorApi error={eliminar.error} /> : null}
      {error ? (
        <ErrorApi error={error} />
      ) : cargando ? (
        <Skeleton className="h-40 rounded-xl" />
      ) : (
        <div className="max-h-[60vh] overflow-auto rounded-xl border bg-card shadow-nova-sm">
          <table className="w-full min-w-[640px] border-separate border-spacing-0 text-sm">
            <thead className="sticky top-0 z-10 bg-nova-hielo text-left text-xs tracking-wide text-nova-gris uppercase">
              <tr>
                {columnas.map((c) => (
                  <th key={c.titulo} className={cn('border-b px-3 py-2.5 font-medium whitespace-nowrap', c.clase)}>
                    {c.titulo}
                  </th>
                ))}
                {(permisos.editar || permisos.eliminar) && <th className="border-b px-3 py-2.5" aria-label="Acciones" />}
              </tr>
            </thead>
            <tbody>
              {(filas ?? []).length === 0 && (
                <tr>
                  <td colSpan={columnas.length + 1} className="px-3 py-8 text-center text-nova-gris">
                    Sin registros todavía.
                  </td>
                </tr>
              )}
              {(filas ?? []).map((t) => {
                const id = idDe(t)
                return (
                  <tr key={id} className="align-top hover:bg-nova-hielo/30">
                    {columnas.map((c) => (
                      <td key={c.titulo} className={cn('border-b px-3 py-2', c.clase)}>
                        {c.valor(t)}
                      </td>
                    ))}
                    {(permisos.editar || permisos.eliminar) && (
                      <td className="border-b px-3 py-1.5 text-right whitespace-nowrap">
                        {borrar === id ? (
                          <span className="inline-flex items-center gap-1.5 text-xs">
                            ¿Eliminar?
                            <Button
                              size="sm"
                              variant="destructive"
                              disabled={eliminar.isPending}
                              onClick={() => eliminar.mutate(id, { onSuccess: () => (setBorrar(null), setHecho('Eliminado.')) })}
                            >
                              Sí
                            </Button>
                            <Button size="sm" variant="ghost" onClick={() => setBorrar(null)}>
                              No
                            </Button>
                          </span>
                        ) : (
                          <>
                            {permisos.editar && (
                              <Button variant="ghost" size="icon" onClick={() => abrir(t)} aria-label={`Editar ${nombreDe(t)}`}>
                                <Pencil />
                              </Button>
                            )}
                            {permisos.eliminar && (
                              <Button
                                variant="ghost"
                                size="icon"
                                className="text-nova-gris hover:text-error"
                                onClick={() => (eliminar.reset(), setBorrar(id))}
                                aria-label={`Eliminar ${nombreDe(t)}`}
                              >
                                <Trash2 />
                              </Button>
                            )}
                          </>
                        )}
                      </td>
                    )}
                  </tr>
                )
              })}
            </tbody>
          </table>
        </div>
      )}
    </section>
  )
}

/** Helpers para pasar de Valores al body. */
export const texto = (v: Valores, k: string) => ((v[k] as string) ?? '').trim()
export const textoONulo = (v: Valores, k: string) => texto(v, k) || null
export const numeroONulo = (v: Valores, k: string) => (texto(v, k) === '' ? null : Number(texto(v, k)))

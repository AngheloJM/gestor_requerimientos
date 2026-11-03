import { ArrowLeft, CheckCircle2, Lock, Trash2 } from 'lucide-react'
import { useMemo, useState } from 'react'
import { Link, useNavigate, useParams } from 'react-router-dom'

import {
  type Alcance,
  type Permiso,
  type RolConfigurable,
  useEliminarRol,
  useGuardarRol,
  usePermisos,
  useRol,
} from '@/api'
import { ErrorApi } from '@/components/ErrorApi'
import { AvisoFase } from '@/components/roles/AvisoFase'
import { Button, buttonVariants } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import {
  ACCIONES,
  ALCANCES,
  BASICAS,
  type Modulo,
  type Recurso,
  type Seleccion,
  aAsignados,
  agrupar,
  alcanceDe,
  alternar,
  aplicarAModulo,
  contar,
  desdeAsignados,
  fijarAlcance,
} from '@/lib/permisos'
import { cn } from '@/lib/utils'

function Casilla({
  permiso,
  marcado,
  soloLectura,
  onChange,
  conTexto,
}: {
  permiso: Permiso
  marcado: boolean
  soloLectura: boolean
  onChange: () => void
  conTexto?: boolean
}) {
  return (
    <label
      title={permiso.descripcion}
      className={cn(
        'inline-flex items-center gap-1.5 rounded-md text-sm',
        conTexto && 'border bg-white px-2 py-1',
        conTexto && marcado && 'border-nova-azul bg-nova-hielo text-nova-azul',
        soloLectura ? 'cursor-default' : 'cursor-pointer',
      )}
    >
      <input
        type="checkbox"
        className="size-4 accent-nova-azul"
        checked={marcado}
        disabled={soloLectura}
        onChange={onChange}
        aria-label={`${permiso.recurso_nombre}: ${ACCIONES[permiso.accion] ?? permiso.accion}`}
      />
      {conTexto && (ACCIONES[permiso.accion] ?? permiso.accion)}
    </label>
  )
}

function FilaRecurso({
  r,
  sel,
  soloLectura,
  setSel,
}: {
  r: Recurso
  sel: Seleccion
  soloLectura: boolean
  setSel: (s: Seleccion) => void
}) {
  const otras = r.permisos.filter((p) => !(BASICAS as readonly string[]).includes(p.accion))
  const algunoConAlcance = r.permisos.some((p) => p.admite_alcance && sel.has(p.clave))
  return (
    <tr className="align-middle hover:bg-nova-hielo/30">
      <th scope="row" className="px-3 py-2.5 text-left font-medium text-nova-titulo">
        {r.nombre}
      </th>
      {BASICAS.map((accion) => {
        const p = r.permisos.find((x) => x.accion === accion)
        return (
          <td key={accion} className="px-3 py-2.5 text-center">
            {p ? (
              <Casilla
                permiso={p}
                marcado={sel.has(p.clave)}
                soloLectura={soloLectura}
                onChange={() => setSel(alternar(sel, p, r))}
              />
            ) : (
              <span className="text-nova-gris/50" aria-hidden>
                —
              </span>
            )}
          </td>
        )
      })}
      <td className="px-3 py-2.5">
        <div className="flex flex-wrap gap-1.5">
          {otras.map((p) => (
            <Casilla
              key={p.clave}
              permiso={p}
              conTexto
              marcado={sel.has(p.clave)}
              soloLectura={soloLectura}
              onChange={() => setSel(alternar(sel, p, r))}
            />
          ))}
        </div>
      </td>
      <td className="px-3 py-2.5">
        {r.admiteAlcance ? (
          <select
            aria-label={`Alcance de ${r.nombre}`}
            className="h-8 rounded-md border border-input bg-white px-2 text-sm disabled:opacity-50"
            value={alcanceDe(sel, r)}
            disabled={soloLectura || !algunoConAlcance}
            onChange={(e) => setSel(fijarAlcance(sel, r, e.target.value as Alcance))}
          >
            {Object.entries(ALCANCES).map(([valor, texto]) => (
              <option key={valor} value={valor}>
                {texto}
              </option>
            ))}
          </select>
        ) : (
          <span className="text-xs text-nova-gris">Global</span>
        )}
      </td>
    </tr>
  )
}

function SeccionModulo({
  m,
  sel,
  soloLectura,
  setSel,
}: {
  m: Modulo
  sel: Seleccion
  soloLectura: boolean
  setSel: (s: Seleccion) => void
}) {
  const { marcados, total } = contar(sel, m)
  return (
    <section className="overflow-hidden rounded-xl border bg-card shadow-nova-sm">
      <header className="flex flex-wrap items-center justify-between gap-3 border-b bg-nova-hielo/50 px-4 py-3">
        <h3 className="font-semibold text-nova-titulo">
          {m.modulo}{' '}
          <span className="ml-1 text-sm font-normal text-nova-gris">
            {marcados} de {total}
          </span>
        </h3>
        {!soloLectura && (
          <div className="flex gap-1.5">
            {(
              [
                ['todo', 'Todo'],
                ['ver', 'Solo ver'],
                ['nada', 'Nada'],
              ] as const
            ).map(([modo, texto]) => (
              <Button key={modo} type="button" size="sm" variant="outline" onClick={() => setSel(aplicarAModulo(sel, m, modo))}>
                {texto}
              </Button>
            ))}
          </div>
        )}
      </header>
      <div className="overflow-x-auto">
        <table className="w-full min-w-[720px] text-sm">
          <thead className="text-xs tracking-wide text-nova-gris uppercase">
            <tr>
              <th className="px-3 py-2 text-left font-medium">Recurso</th>
              {BASICAS.map((a) => (
                <th key={a} className="w-20 px-3 py-2 text-center font-medium">
                  {ACCIONES[a]}
                </th>
              ))}
              <th className="px-3 py-2 text-left font-medium">Otras acciones</th>
              <th className="w-40 px-3 py-2 text-left font-medium">Alcance</th>
            </tr>
          </thead>
          <tbody className="divide-y">
            {m.recursos.map((r) => (
              <FilaRecurso key={r.recurso} r={r} sel={sel} soloLectura={soloLectura} setSel={setSel} />
            ))}
          </tbody>
        </table>
      </div>
    </section>
  )
}

function Editor({ rol, catalogo }: { rol?: RolConfigurable; catalogo: Permiso[] }) {
  const navigate = useNavigate()
  const guardar = useGuardarRol(rol?.id)
  const eliminar = useEliminarRol()
  const soloLectura = !!rol?.es_sistema
  const modulos = useMemo(() => agrupar(catalogo), [catalogo])

  const [nombre, setNombre] = useState(rol?.nombre ?? '')
  const [descripcion, setDescripcion] = useState(rol?.descripcion ?? '')
  const [activo, setActivo] = useState(rol?.activo ?? true)
  const [sel, setSel] = useState<Seleccion>(() => desdeAsignados(rol?.permisos ?? []))
  const [errorNombre, setErrorNombre] = useState<string>()
  const [guardado, setGuardado] = useState(false)
  const [confirmarBaja, setConfirmarBaja] = useState(false)

  const cambiar = (s: Seleccion) => {
    setSel(s)
    setGuardado(false)
  }

  const enviar = (e: React.FormEvent) => {
    e.preventDefault()
    if (nombre.trim().length < 2) {
      setErrorNombre('Indicá un nombre de al menos 2 caracteres.')
      return
    }
    setErrorNombre(undefined)
    guardar.mutate(
      { nombre: nombre.trim(), descripcion: descripcion.trim() || null, activo, permisos: aAsignados(sel, catalogo) },
      {
        onSuccess: (r) => {
          setGuardado(true)
          if (!rol) navigate(`/roles/${r.id}`, { replace: true })
        },
      },
    )
  }

  return (
    <form onSubmit={enviar} className="grid gap-5">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <Link to="/roles" className={buttonVariants({ variant: 'ghost', size: 'icon' })} aria-label="Volver a roles">
            <ArrowLeft />
          </Link>
          <h2 className="text-2xl font-semibold text-nova-titulo">{rol ? rol.nombre : 'Crear rol'}</h2>
        </div>
        <p className="text-sm text-nova-gris">
          {sel.size} de {catalogo.length} permisos marcados
        </p>
      </div>

      {soloLectura ? (
        <div className="flex gap-3 rounded-xl border bg-nova-hielo/60 p-4 text-sm text-nova-texto-2">
          <Lock className="mt-0.5 size-4 shrink-0 text-nova-azul" aria-hidden />
          <p>
            <strong className="text-nova-titulo">Rol de sistema.</strong> Muestra lo que hoy permiten las reglas de la base
            para este rol; por eso no se edita aquí. Para otro conjunto de permisos, creá un rol nuevo.
          </p>
        </div>
      ) : (
        <AvisoFase />
      )}

      <section className="grid gap-4 rounded-xl border bg-card p-5 shadow-nova-sm sm:grid-cols-[1fr_2fr_auto]">
        <div className="grid gap-1.5">
          <Label htmlFor="nombre">Nombre del rol</Label>
          <Input
            id="nombre"
            className="h-9 bg-white"
            maxLength={80}
            value={nombre}
            disabled={soloLectura}
            aria-invalid={!!errorNombre}
            onChange={(e) => {
              setNombre(e.target.value)
              setGuardado(false)
            }}
            placeholder="Ej.: Gerente"
          />
          {errorNombre && (
            <p className="text-xs text-error" role="alert">
              {errorNombre}
            </p>
          )}
        </div>
        <div className="grid gap-1.5">
          <Label htmlFor="descripcion">Descripción (opcional)</Label>
          <Input
            id="descripcion"
            className="h-9 bg-white"
            maxLength={500}
            value={descripcion}
            disabled={soloLectura}
            onChange={(e) => {
              setDescripcion(e.target.value)
              setGuardado(false)
            }}
            placeholder="Para qué sirve este rol"
          />
        </div>
        <label className="flex items-center gap-2 self-end pb-2 text-sm">
          <input
            type="checkbox"
            className="size-4 accent-nova-azul"
            checked={activo}
            disabled={soloLectura}
            onChange={(e) => {
              setActivo(e.target.checked)
              setGuardado(false)
            }}
          />
          Activo
        </label>
      </section>

      {modulos.map((m) => (
        <SeccionModulo key={m.modulo} m={m} sel={sel} soloLectura={soloLectura} setSel={cambiar} />
      ))}

      {guardar.error && <ErrorApi error={guardar.error} />}
      {eliminar.error && <ErrorApi error={eliminar.error} />}

      {!soloLectura && (
        <div className="sticky bottom-0 -mx-4 flex flex-wrap items-center justify-between gap-3 border-t bg-white/95 px-4 py-3 backdrop-blur lg:-mx-8 lg:px-8">
          <div>
            {rol &&
              (confirmarBaja ? (
                <span className="flex flex-wrap items-center gap-2 text-sm">
                  ¿Eliminar el rol “{rol.nombre}”?
                  <Button
                    type="button"
                    variant="destructive"
                    size="sm"
                    disabled={eliminar.isPending}
                    onClick={() => eliminar.mutate(rol.id, { onSuccess: () => navigate('/roles', { replace: true }) })}
                  >
                    Sí, eliminar
                  </Button>
                  <Button type="button" variant="ghost" size="sm" onClick={() => setConfirmarBaja(false)}>
                    No
                  </Button>
                </span>
              ) : (
                <Button type="button" variant="ghost" className="text-error" onClick={() => setConfirmarBaja(true)}>
                  <Trash2 aria-hidden /> Eliminar rol
                </Button>
              ))}
          </div>
          <div className="flex items-center gap-3">
            {guardado && !guardar.isPending && (
              <span className="flex items-center gap-1.5 text-sm text-exito" role="status">
                <CheckCircle2 className="size-4" aria-hidden /> Guardado
              </span>
            )}
            <Link to="/roles" className={buttonVariants({ variant: 'outline' })}>
              Cancelar
            </Link>
            <Button type="submit" disabled={guardar.isPending}>
              {guardar.isPending ? 'Guardando…' : rol ? 'Guardar cambios' : 'Crear rol'}
            </Button>
          </div>
        </div>
      )}
    </form>
  )
}

export function RolEditorPage() {
  const { id } = useParams()
  const nuevo = id === 'nuevo'
  const catalogo = usePermisos()
  const rol = useRol(nuevo ? undefined : id)

  const error = catalogo.error ?? rol.error
  if (error) return <ErrorApi error={error} />
  if (catalogo.isPending || (!nuevo && rol.isPending)) {
    return (
      <div className="grid gap-4">
        {[0, 1, 2].map((i) => (
          <Skeleton key={i} className="h-40 rounded-xl" />
        ))}
      </div>
    )
  }
  // `key` reinicia el estado del editor al cambiar de rol.
  return <Editor key={rol.data?.id ?? 'nuevo'} rol={nuevo ? undefined : rol.data} catalogo={catalogo.data!} />
}

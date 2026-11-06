import { zodResolver } from '@hookform/resolvers/zod'
import { ArrowLeft, CheckCircle2, ClipboardCheck, Info, Plus, Trash2 } from 'lucide-react'
import { useEffect, useMemo, useState } from 'react'
import { useFieldArray, useForm, useWatch } from 'react-hook-form'
import { Link } from 'react-router-dom'
import { z } from 'zod'

import {
  type CargoOpcion,
  type CrearRequerimiento,
  type TipoRequerimiento,
  useCrearRequerimiento,
  useMe,
  useOpcionesRequerimiento,
} from '@/api'
import { ErrorApi } from '@/components/ErrorApi'
import { PanelServicio } from '@/components/requerimiento/PanelServicio'
import { PanelSolicitante } from '@/components/requerimiento/PanelSolicitante'
import { Button, buttonVariants } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import { capacitacionCalculada } from '@/lib/capacitacion'
import { formatFecha, formatMonto } from '@/lib/format'
import { cn } from '@/lib/utils'

const TIPOS: { valor: TipoRequerimiento; titulo: string; detalle: string }[] = [
  { valor: 'nuevo', titulo: 'Nuevo', detalle: 'Personal para un puesto' },
  { valor: 'modificacion', titulo: 'Modificación', detalle: 'Cambia un pedido existente' },
  { valor: 'promocion', titulo: 'Promoción', detalle: 'Ascenso interno' },
  { valor: 'cambio', titulo: 'Cambio', detalle: 'Reemplazo o traslado' },
]

/** Valor del selector de servicio para cargos administrativo_staff (sin servicio). */
const SIN_SERVICIO = '__staff__'
const FECHA = /^\d{4}-\d{2}-\d{2}$/
const MONTO = /^\d{1,9}(\.\d{1,2})?$/

const esquema = z.object({
  tipo: z.enum(['nuevo', 'modificacion', 'promocion', 'cambio'], { error: 'Elegí el tipo de requerimiento.' }),
  servicio: z.string().min(1, 'Elegí un servicio.'),
  cargo_id: z.string().min(1, 'Elegí un cargo.'),
  cantidad: z.string().regex(/^[1-9]\d{0,4}$/, 'Indicá al menos 1 persona.'),
  motivo: z.string().min(1, 'Elegí un motivo.'),
  fecha_inicio_operacion: z.string().regex(FECHA, 'Indicá la fecha de inicio de operación.'),
  fecha_inicio_capacitacion: z.string(),
  fecha_fin: z.string(),
  comentarios: z.string().max(2000, 'Máximo 2000 caracteres.'),
  bonos_extra: z
    .array(
      z.object({
        nombre: z.string().trim().min(1, 'Indicá el nombre.').max(200),
        monto: z
          .string()
          .regex(MONTO, 'Monto con hasta 2 decimales.')
          .refine((v) => Number(v) > 0, 'Debe ser mayor a 0.'),
        motivo: z.string().trim().min(1, 'Indicá el motivo.').max(500),
      }),
    )
    .max(20, 'Como máximo 20 bonos extra.'),
})

type Formulario = z.infer<typeof esquema>

const VACIO: Formulario = {
  tipo: undefined as unknown as TipoRequerimiento,
  servicio: '',
  cargo_id: '',
  cantidad: '1',
  motivo: '',
  fecha_inicio_operacion: '',
  fecha_inicio_capacitacion: '',
  fecha_fin: '',
  comentarios: '',
  bonos_extra: [],
}

const selectClase =
  'h-9 w-full rounded-lg border border-input bg-white px-2.5 text-sm outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 aria-invalid:border-destructive disabled:opacity-50'

function Campo({
  id,
  etiqueta,
  error,
  ayuda,
  children,
  className,
}: {
  id: string
  etiqueta: string
  error?: string
  ayuda?: React.ReactNode
  children: React.ReactNode
  className?: string
}) {
  return (
    <div className={cn('grid gap-1.5', className)}>
      <Label htmlFor={id}>{etiqueta}</Label>
      {children}
      {error ? (
        <p className="text-xs text-error" role="alert">
          {error}
        </p>
      ) : (
        ayuda && <p className="text-xs text-nova-gris">{ayuda}</p>
      )}
    </div>
  )
}

function Seccion({ titulo, children }: { titulo: string; children: React.ReactNode }) {
  return (
    <section className="rounded-xl border bg-card p-5 shadow-nova-sm">
      <h3 className="mb-4 text-base font-semibold text-nova-titulo">{titulo}</h3>
      {children}
    </section>
  )
}

function Exito({ folio, otro }: { folio: string; otro: () => void }) {
  return (
    <div className="mx-auto grid max-w-lg place-items-center rounded-xl border bg-card px-6 py-12 text-center shadow-nova-sm">
      <span className="grid size-12 place-items-center rounded-full bg-exito/10 text-exito">
        <CheckCircle2 className="size-6" aria-hidden />
      </span>
      <h2 className="mt-4 text-xl font-semibold text-nova-titulo">Requerimiento {folio} creado</h2>
      <p className="mt-1 text-sm text-nova-gris">
        Quedó en estado <strong>pendiente</strong>. Vas a ver en “Mis requerimientos” cuando se apruebe o rechace.
      </p>
      <div className="mt-6 flex flex-wrap justify-center gap-2">
        <Link to="/requerimientos" className={buttonVariants()}>
          Ver mis requerimientos
        </Link>
        <Button variant="outline" onClick={otro}>
          Crear otro
        </Button>
      </div>
    </div>
  )
}

const ETIQUETA_TIPO: Record<string, string> = { nuevo: 'Nuevo', modificacion: 'Modificación', promocion: 'Promoción', cambio: 'Cambio' }

function Resumen({
  r,
  servicio,
  cargo,
  creando,
  error,
  volver,
  confirmar,
}: {
  r: CrearRequerimiento
  servicio: string
  cargo: string
  creando: boolean
  error: unknown
  volver: () => void
  confirmar: () => void
}) {
  const filas: [string, React.ReactNode][] = [
    ['Tipo', ETIQUETA_TIPO[r.tipo] ?? r.tipo],
    ['Cantidad de personas', r.cantidad],
    ['Motivo', r.motivo],
    ['Servicio', servicio],
    ['Cargo', cargo],
    ['Fecha de inicio de operación', formatFecha(r.fecha_inicio_operacion)],
  ]
  return (
    <section className="grid min-w-0 gap-5 rounded-xl border border-nova-azul/40 bg-card p-6 shadow-nova-md" aria-label="Resumen del requerimiento">
      <div className="flex items-start gap-3">
        <ClipboardCheck className="mt-0.5 size-5 text-nova-azul" aria-hidden />
        <div>
          <h3 className="text-lg font-semibold text-nova-titulo">Revisá el requerimiento antes de crearlo</h3>
          <p className="text-sm text-nova-gris">Todavía no se guardó. Al confirmar se congelan el sueldo y los bonos del cargo.</p>
        </div>
      </div>
      <dl className="grid gap-x-6 gap-y-4 sm:grid-cols-2 lg:grid-cols-3">
        {filas.map(([t, v]) => (
          <div key={t}>
            <dt className="text-xs text-nova-gris">{t}</dt>
            <dd className="mt-0.5 text-sm font-medium text-nova-titulo">{v}</dd>
          </div>
        ))}
      </dl>
      <div>
        <p className="text-xs text-nova-gris">Bonos extra</p>
        {r.bonos_extra.length === 0 ? (
          <p className="mt-0.5 text-sm text-nova-titulo">Ninguno</p>
        ) : (
          <ul className="mt-1 divide-y rounded-lg border">
            {r.bonos_extra.map((b, i) => (
              <li key={i} className="flex flex-wrap justify-between gap-2 px-3 py-2 text-sm">
                <span>
                  {b.nombre} <span className="text-nova-gris">· {b.motivo}</span>
                </span>
                <span className="tabular-nums">Bs {formatMonto(b.monto)}</span>
              </li>
            ))}
          </ul>
        )}
      </div>
      <div>
        <p className="text-xs text-nova-gris">Comentarios adicionales</p>
        <p className="mt-0.5 text-sm whitespace-pre-wrap text-nova-titulo">{r.comentarios ?? 'Sin comentarios'}</p>
      </div>
      {error ? <ErrorApi error={error} /> : null}
      <div className="flex flex-wrap justify-end gap-2">
        <Button type="button" variant="outline" onClick={volver} disabled={creando}>
          <ArrowLeft aria-hidden /> Volver a editar
        </Button>
        <Button type="button" onClick={confirmar} disabled={creando}>
          {creando ? 'Creando…' : 'Confirmar y crear'}
        </Button>
      </div>
    </section>
  )
}

export function NuevoRequerimientoPage() {
  const opciones = useOpcionesRequerimiento()
  const { data: me } = useMe()
  const crear = useCrearRequerimiento()
  const {
    register,
    handleSubmit,
    control,
    setValue,
    reset,
    formState: { errors },
  } = useForm<Formulario>({ resolver: zodResolver(esquema), defaultValues: VACIO })
  const bonos = useFieldArray({ control, name: 'bonos_extra' })
  // Paso de confirmación: el formulario válido muestra un resumen; recién ahí se crea.
  const [resumen, setResumen] = useState<CrearRequerimiento | null>(null)

  const [tipo, servicio, cargoId, operacion, motivo] = useWatch({
    control,
    name: ['tipo', 'servicio', 'cargo_id', 'fecha_inicio_operacion', 'motivo'],
  })

  const datos = opciones.data
  const servicioSel = datos?.servicios.find((s) => s.id === servicio)
  const cargos: CargoOpcion[] = useMemo(
    () => (servicio === SIN_SERVICIO ? (datos?.cargos_staff ?? []) : (servicioSel?.cargos ?? [])),
    [servicio, servicioSel, datos],
  )
  const cargoSel = cargos.find((c) => c.id === cargoId)
  const dias = servicio === SIN_SERVICIO ? null : servicioSel?.tiempo_capacitacion_dias
  const calculada = capacitacionCalculada(cargoSel?.tipo, tipo, dias, operacion)

  // Al cambiar de servicio, el cargo elegido deja de ser válido.
  useEffect(() => {
    if (cargoId && !cargos.some((c) => c.id === cargoId)) setValue('cargo_id', '')
  }, [cargos, cargoId, setValue])

  const enviar = handleSubmit((f) => {
    // Solo se pide la fecha de operación (revisión con la clienta). La base exige además la de
    // capacitación: si el cargo es operativo y el servicio tiene días, la calcula ella misma
    // (operación − días); si no, se envía la misma fecha de operación.
    // A confirmar con la clienta: capacitación = operación cuando no hay días que restar.
    const capacitacion = calculada ?? f.fecha_inicio_operacion
    const body: CrearRequerimiento = {
      tipo: f.tipo,
      servicio_id: f.servicio === SIN_SERVICIO ? null : f.servicio,
      cargo_id: f.cargo_id,
      cantidad: Number(f.cantidad),
      motivo: f.motivo,
      fecha_inicio_operacion: f.fecha_inicio_operacion,
      fecha_inicio_capacitacion: capacitacion,
      fecha_fin: null,
      comentarios: f.comentarios.trim() || null,
      bonos_extra: f.bonos_extra.map((b) => ({ nombre: b.nombre.trim(), monto: b.monto, motivo: b.motivo.trim() })),
    }
    crear.reset()
    setResumen(body)
    window.scrollTo({ top: 0, behavior: 'smooth' })
  })

  if (crear.data) {
    return (
      <Exito
        folio={crear.data.folio}
        otro={() => {
          crear.reset()
          setResumen(null)
          reset(VACIO)
        }}
      />
    )
  }

  if (opciones.error) return <ErrorApi error={opciones.error} />
  if (opciones.isPending || !datos) {
    return (
      <div className="grid gap-4">
        {[0, 1, 2].map((i) => (
          <Skeleton key={i} className="h-40 rounded-xl" />
        ))}
      </div>
    )
  }

  const sinServicios = datos.servicios.length === 0 && datos.cargos_staff.length === 0

  const sinServicio = servicio === SIN_SERVICIO

  return (
    <div className="grid gap-5">
      <div>
        <h2 className="text-2xl font-semibold text-nova-titulo">Nuevo requerimiento</h2>
        <p className="mt-1 text-sm text-nova-gris">
          Al guardarlo se congelan el sueldo y los bonos vigentes del cargo; queda pendiente de aprobación por RR.HH.
        </p>
      </div>

      <div className="grid items-start gap-5 lg:grid-cols-[minmax(0,1fr)_21rem]">
        {resumen && (
          <Resumen
            r={resumen}
            servicio={sinServicio ? 'Sin servicio (staff)' : (servicioSel?.nombre ?? '—')}
            cargo={cargoSel?.nombre ?? '—'}
            creando={crear.isPending}
            error={crear.error}
            volver={() => {
              crear.reset()
              setResumen(null)
            }}
            confirmar={() => crear.mutate(resumen)}
          />
        )}
        {/* Oculto (no desmontado) durante el resumen, para no perder lo cargado. */}
        <form onSubmit={enviar} noValidate className="grid min-w-0 gap-5" hidden={!!resumen}>
          {sinServicios && (
            <div className="flex gap-3 rounded-xl border border-advertencia/40 bg-advertencia/10 p-4 text-sm">
              <Info className="mt-0.5 size-4 shrink-0 text-[#B45309]" aria-hidden />
              No tenés servicios asignados con cargos configurados. Pedile a RR.HH. que te asigne uno.
            </div>
          )}

          <Seccion titulo="1. Tipo de requerimiento">
            <div role="radiogroup" aria-label="Tipo" className="grid gap-3 sm:grid-cols-2">
              {TIPOS.filter((t) => datos.tipos.includes(t.valor)).map((t) => (
                <label
                  key={t.valor}
                  className={cn(
                    'cursor-pointer rounded-lg border bg-white p-3 transition-colors hover:border-nova-azul-medio',
                    tipo === t.valor && 'border-nova-azul bg-nova-hielo ring-1 ring-nova-azul',
                  )}
                >
                  <input type="radio" value={t.valor} className="sr-only" {...register('tipo')} />
                  <span className="block font-medium text-nova-titulo">{t.titulo}</span>
                  <span className="block text-xs text-nova-gris">{t.detalle}</span>
                </label>
              ))}
            </div>
            {errors.tipo && (
              <p className="mt-2 text-xs text-error" role="alert">
                {errors.tipo.message}
              </p>
            )}
            <div className="mt-4 grid gap-4 sm:grid-cols-2">
              <Campo id="cantidad" etiqueta="Cantidad de personas requeridas" error={errors.cantidad?.message}>
                <Input id="cantidad" inputMode="numeric" className="h-9 bg-white" aria-invalid={!!errors.cantidad} {...register('cantidad')} />
              </Campo>
              <Campo id="motivo" etiqueta="Motivo del requerimiento" error={errors.motivo?.message}>
                <select id="motivo" className={selectClase} aria-invalid={!!errors.motivo} {...register('motivo')}>
                  <option value="">Elegí un motivo…</option>
                  {datos.motivos.map((m) => (
                    <option key={m} value={m}>
                      {m}
                    </option>
                  ))}
                </select>
              </Campo>
            </div>
          </Seccion>

          <Seccion titulo="2. Servicio y cargo">
            <div className="grid gap-4 sm:grid-cols-2">
              <Campo id="servicio" etiqueta="Servicio" error={errors.servicio?.message}>
                <select id="servicio" className={selectClase} aria-invalid={!!errors.servicio} {...register('servicio')}>
                  <option value="">Elegí un servicio…</option>
                  {datos.servicios.map((s) => (
                    <option key={s.id} value={s.id}>
                      {s.nombre}
                    </option>
                  ))}
                  {datos.cargos_staff.length > 0 && <option value={SIN_SERVICIO}>Sin servicio (cargo administrativo)</option>}
                </select>
              </Campo>
              <Campo id="cargo_id" etiqueta="Cargo" error={errors.cargo_id?.message} ayuda={servicio ? undefined : 'Primero elegí el servicio.'}>
                <select id="cargo_id" className={selectClase} disabled={!servicio} aria-invalid={!!errors.cargo_id} {...register('cargo_id')}>
                  <option value="">Elegí un cargo…</option>
                  {cargos.map((c) => (
                    <option key={c.id} value={c.id}>
                      {c.nombre}
                    </option>
                  ))}
                </select>
              </Campo>
            </div>
            {/* Crecimiento: solo referencia informativa, no se autocompleta nada.
                TODO(negocio): confirmar con la clienta si quiere algo más específico. */}
            {motivo === 'Crecimiento' && cargoSel && servicio !== SIN_SERVICIO && (
              <p className="rounded-lg border border-info/30 bg-info/5 px-3 py-2 text-sm text-nova-texto-2" role="note">
                {cargoSel.cantidad_mujeres == null && cargoSel.cantidad_hombres == null
                  ? 'Este cargo no tiene configuradas cantidades de mujeres y hombres.'
                  : `Este cargo tiene configurado: ${cargoSel.cantidad_mujeres ?? 0} mujer${cargoSel.cantidad_mujeres === 1 ? '' : 'es'}, ${cargoSel.cantidad_hombres ?? 0} hombre${cargoSel.cantidad_hombres === 1 ? '' : 's'}.`}
              </p>
            )}
          </Seccion>

          <Seccion titulo="3. Bonos extra (opcional)">
            <p className="mb-3 text-sm text-nova-gris">
              Los bonos fijos del servicio y cargo se incluyen solos (ver panel derecho). Agregá aquí solo excepciones.
            </p>
            <div className="grid gap-3">
              {bonos.fields.map((b, i) => {
                const e = errors.bonos_extra?.[i]
                return (
                  <div key={b.id} className="grid items-start gap-3 rounded-lg border bg-white p-3 sm:grid-cols-[1fr_9rem_1fr_auto]">
                    <Campo id={`bono-${i}-nombre`} etiqueta="Nombre" error={e?.nombre?.message}>
                      <Input id={`bono-${i}-nombre`} className="h-9" {...register(`bonos_extra.${i}.nombre`)} />
                    </Campo>
                    <Campo id={`bono-${i}-monto`} etiqueta="Monto (Bs)" error={e?.monto?.message}>
                      <Input id={`bono-${i}-monto`} inputMode="decimal" placeholder="0.00" className="h-9 tabular-nums" {...register(`bonos_extra.${i}.monto`)} />
                    </Campo>
                    <Campo id={`bono-${i}-motivo`} etiqueta="Motivo" error={e?.motivo?.message}>
                      <Input id={`bono-${i}-motivo`} className="h-9" {...register(`bonos_extra.${i}.motivo`)} />
                    </Campo>
                    <Button type="button" variant="ghost" size="icon" className="mt-6 text-nova-gris hover:text-error" onClick={() => bonos.remove(i)} aria-label={`Quitar bono ${i + 1}`}>
                      <Trash2 />
                    </Button>
                  </div>
                )
              })}
              {bonos.fields.length < 20 && (
                <Button type="button" variant="outline" className="justify-self-start" onClick={() => bonos.append({ nombre: '', monto: '', motivo: '' })}>
                  <Plus aria-hidden /> Agregar bono extra
                </Button>
              )}
            </div>
          </Seccion>

          <Seccion titulo="4. Datos adicionales">
            <div className="grid gap-4 sm:grid-cols-3">
              <Campo id="fecha_inicio_operacion" etiqueta="Fecha de inicio de operación" error={errors.fecha_inicio_operacion?.message}>
                <Input id="fecha_inicio_operacion" type="date" className="h-9 bg-white" aria-invalid={!!errors.fecha_inicio_operacion} {...register('fecha_inicio_operacion')} />
              </Campo>
            </div>
            <div className="mt-4 grid gap-1.5">
              <Label htmlFor="comentarios">Comentarios adicionales</Label>
              <textarea
                id="comentarios"
                rows={3}
                className="w-full rounded-lg border border-input bg-white px-2.5 py-2 text-sm outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
                {...register('comentarios')}
              />
              {errors.comentarios && <p className="text-xs text-error">{errors.comentarios.message}</p>}
            </div>
          </Seccion>

          <div className="flex flex-wrap justify-end gap-2 pb-4">
            <Link to="/requerimientos" className={buttonVariants({ variant: 'outline' })}>
              Cancelar
            </Link>
            <Button type="submit" disabled={sinServicios}>
              Revisar y crear
            </Button>
          </div>
        </form>

        <div className="grid min-w-0 gap-5">
          <PanelSolicitante
            datos={{ nombre: me?.nombre, cargo: me?.cargo, division: me?.division, region: me?.region, inmediato_superior: me?.inmediato_superior }}
          />
          <PanelServicio servicioId={sinServicio ? '' : servicio} cargoId={cargoId} sinServicio={sinServicio} />
        </div>
      </div>
    </div>
  )
}

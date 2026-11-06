import {
  type CargoCfg,
  type CentroCostoCfg,
  type ClienteCfg,
  type ContratoCfg,
  type ProyectoCfg,
  type ServicioCfg,
  useCatalogosServicio,
  useConfigLista,
} from '@/api'
import { EstadoBadge } from '@/components/EstadoBadge'
import { CargosServicio } from '@/components/matriz/CargosServicio'
import { CrudSeccion, numeroONulo, texto, textoONulo, type Valores } from '@/components/matriz/CrudSeccion'
import { DEPARTAMENTOS } from '@/lib/catalogos'
import { formatFecha } from '@/lib/format'

type Permisos = { esRrhh: boolean }
type ConApertura = Permisos & { abrirId?: string | null }
const todo = (esRrhh: boolean) => ({ crear: esRrhh, editar: esRrhh, eliminar: esRrhh })
const vacio = (v: string | null | undefined) => v || '—'
const NUMERO_ENTERO = /^\d+$/
const opcionesDias = (): [string, string][] => Array.from({ length: 30 }, (_, i) => [String(i + 1), `${i + 1} día${i ? 's' : ''}`])

/**
 * Proyecto se oculta del front (revisión 04/11/2026): todavía no se usa. La tabla `proyectos`,
 * su relación con cliente y `servicios.proyecto_id` se mantienen intactos en la base y en la API.
 */
const MOSTRAR_PROYECTOS = false

// --- 1. Clientes y proyectos ---------------------------------------------------------------

export function ClientesYProyectos({ esRrhh }: Permisos) {
  const clientes = useConfigLista<ClienteCfg>('clientes')
  const proyectos = useConfigLista<ProyectoCfg>('proyectos')
  const opcionesClientes = (): [string, string][] => (clientes.data ?? []).map((c) => [c.id, c.nombre])
  return (
    <div className="grid gap-8">
      <CrudSeccion<ClienteCfg>
        titulo="Clientes"
        // A confirmar con la clienta: redacción más formal del alta.
        tituloNuevo="Registro de nuevo cliente"
        recurso="clientes"
        filas={clientes.data}
        cargando={clientes.isPending}
        error={clientes.error}
        permisos={todo(esRrhh)}
        idDe={(c) => c.id}
        nombreDe={(c) => c.nombre}
        nota="Logo y manual de marca se cargan como enlace (URL). Subir el archivo en sí: pendiente (TODO)."
        columnas={[
          { titulo: 'Nombre', valor: (c) => <span className="font-medium text-nova-titulo">{c.nombre}</span> },
          { titulo: 'Logo', valor: (c) => vacio(c.logo_url), clase: 'max-w-64 truncate' },
          { titulo: 'Manual de marca', valor: (c) => vacio(c.manual_marca_url), clase: 'max-w-64 truncate' },
        ]}
        campos={[
          { nombre: 'nombre', etiqueta: 'Nombre', tipo: 'texto', requerido: true },
          { nombre: 'logo_url', etiqueta: 'Logo (URL)', tipo: 'texto' },
          { nombre: 'manual_marca_url', etiqueta: 'Manual de marca (URL)', tipo: 'texto' },
        ]}
        inicial={(c) => ({ nombre: c?.nombre ?? '', logo_url: c?.logo_url ?? '', manual_marca_url: c?.manual_marca_url ?? '' })}
        aBody={(v) => ({ nombre: texto(v, 'nombre'), logo_url: textoONulo(v, 'logo_url'), manual_marca_url: textoONulo(v, 'manual_marca_url') })}
      />
      {MOSTRAR_PROYECTOS && (
        <CrudSeccion<ProyectoCfg>
          titulo="Proyectos"
          recurso="proyectos"
          filas={proyectos.data}
          cargando={proyectos.isPending}
          error={proyectos.error}
          // TODO(negocio): producción no tiene política DELETE en `proyectos`; la baja no está habilitada.
          permisos={{ crear: esRrhh, editar: esRrhh, eliminar: false }}
          nota="Eliminar proyectos no está habilitado: la base de producción no tiene permiso de borrado para esta tabla (pendiente)."
          idDe={(p) => p.id}
          nombreDe={(p) => p.nombre}
          columnas={[
            { titulo: 'Proyecto', valor: (p) => <span className="font-medium text-nova-titulo">{p.nombre}</span> },
            { titulo: 'Cliente', valor: (p) => vacio(p.cliente_nombre) },
          ]}
          campos={[
            { nombre: 'cliente_id', etiqueta: 'Cliente', tipo: 'select', requerido: true, opciones: opcionesClientes },
            { nombre: 'nombre', etiqueta: 'Nombre', tipo: 'texto', requerido: true },
          ]}
          inicial={(p) => ({ cliente_id: p?.cliente_id ?? '', nombre: p?.nombre ?? '' })}
          aBody={(v) => ({ cliente_id: texto(v, 'cliente_id'), nombre: texto(v, 'nombre') })}
        />
      )}
    </div>
  )
}

// --- 2. Centros de costo -----------------------------------------------------------------

export function CentrosCosto({ esRrhh }: Permisos) {
  const q = useConfigLista<CentroCostoCfg>('centros-costo')
  return (
    <CrudSeccion<CentroCostoCfg>
      titulo="Centros de costo"
      descripcion="Independientes del cliente."
      recurso="centros-costo"
      filas={q.data}
      cargando={q.isPending}
      error={q.error}
      // Solo selección (confirmado 2026-11-06): sin alta desde la app.
      permisos={{ crear: false, editar: esRrhh, eliminar: esRrhh }}
      idDe={(c) => c.id}
      nombreDe={(c) => `${c.codigo} · ${c.nombre}`}
      columnas={[
        { titulo: 'Código', valor: (c) => <span className="font-medium text-nova-titulo">{c.codigo}</span> },
        { titulo: 'Nombre', valor: (c) => c.nombre },
      ]}
      campos={[
        { nombre: 'codigo', etiqueta: 'Código', tipo: 'texto', requerido: true },
        { nombre: 'nombre', etiqueta: 'Nombre', tipo: 'texto', requerido: true },
      ]}
      inicial={(c) => ({ codigo: c?.codigo ?? '', nombre: c?.nombre ?? '' })}
      aBody={(v) => ({ codigo: texto(v, 'codigo'), nombre: texto(v, 'nombre') })}
    />
  )
}

// --- 3. Servicios ----------------------------------------------------------------------

export function Servicios({ esRrhh, abrirId }: ConApertura) {
  const q = useConfigLista<ServicioCfg>('servicios')
  const centros = useConfigLista<CentroCostoCfg>('centros-costo')
  const contratos = useConfigLista<ContratoCfg>('contratos')
  const clientes = useConfigLista<ClienteCfg>('clientes')
  const proyectos = useConfigLista<ProyectoCfg>('proyectos')
  const cat = useCatalogosServicio()
  const pares = (xs: string[] | undefined): [string, string][] => (xs ?? []).map((x) => [x, x])

  return (
    <CrudSeccion<ServicioCfg>
      titulo="Servicios"
      descripcion="Cliente y centro de costo se eligen por separado: uno no implica al otro."
      recurso="servicios"
      filas={q.data}
      cargando={q.isPending}
      error={q.error}
      permisos={todo(esRrhh)}
      nota="Para agregar cargos (composición salarial y métricas), editá el servicio. Un servicio con cargos o requerimientos no se puede eliminar (la base lo impide): desactivalo con el interruptor “Activo”."
      idDe={(s) => s.id}
      abrirId={abrirId}
      nombreDe={(s) => s.nombre}
      columnas={[
        { titulo: 'Código', valor: (s) => vacio(s.codigo), clase: 'whitespace-nowrap' },
        { titulo: 'Servicio', valor: (s) => <span className="font-medium text-nova-titulo">{s.nombre}</span>, clase: 'whitespace-nowrap' },
        { titulo: 'Centro de costo', valor: (s) => vacio(s.centro_costo_nombre), clase: 'whitespace-nowrap' },
        { titulo: 'Cliente', valor: (s) => vacio(s.cliente_nombre), clase: 'whitespace-nowrap' },
        { titulo: 'Contrato', valor: (s) => vacio(s.contrato_nombre), clase: 'whitespace-nowrap' },
        { titulo: 'Modalidad', valor: (s) => s.modalidad, clase: 'whitespace-nowrap' },
        { titulo: 'Días', valor: (s) => s.dias_trabajo.map((d) => d.slice(0, 3)).join(', ') || '—', clase: 'whitespace-nowrap' },
        { titulo: 'Capacitación', valor: (s) => (s.tiempo_capacitacion_dias != null ? `${s.tiempo_capacitacion_dias} d` : '—') },
        {
          titulo: 'Estado',
          valor: (s) => (s.activo ? <EstadoBadge tono="exito">Activo</EstadoBadge> : <EstadoBadge tono="error">Inactivo</EstadoBadge>),
        },
      ]}
      campos={[
        { nombre: 'codigo', etiqueta: 'Código', tipo: 'texto', ayuda: 'Opcional; si se carga, único.' },
        { nombre: 'nombre', etiqueta: 'Nombre', tipo: 'texto', requerido: true },
        {
          nombre: 'centro_costo_id',
          etiqueta: 'Centro de costo',
          tipo: 'select',
          opciones: () => (centros.data ?? []).map((c) => [c.id, `${c.codigo} · ${c.nombre}`]),
        },
        {
          nombre: 'cliente_id',
          etiqueta: 'Cliente',
          tipo: 'select',
          opciones: () => (clientes.data ?? []).map((c) => [c.id, c.nombre]),
          // El proyecto debe ser del cliente elegido (regla de la base): se limpia si deja de serlo.
          alCambiar: (v) =>
            (proyectos.data ?? []).some((p) => p.id === v.proyecto_id && p.cliente_id === v.cliente_id) ? v : { ...v, proyecto_id: '' },
        },
        // Proyecto oculto (MOSTRAR_PROYECTOS): `proyecto_id` se conserva tal cual al editar.
        {
          nombre: 'contrato_comercial_id',
          etiqueta: 'Contrato comercial',
          tipo: 'select',
          opciones: () => (contratos.data ?? []).map((k) => [k.id, k.nombre ? `${k.codigo} · ${k.nombre}` : k.codigo]),
        },
        { nombre: 'tipo_contrato', etiqueta: 'Tipo de contrato', tipo: 'select', opciones: () => pares(cat.data?.tipos_contrato) },
        { nombre: 'modalidad', etiqueta: 'Modalidad', tipo: 'select', requerido: true, opciones: () => pares(cat.data?.modalidades) },
        {
          nombre: 'tipo_turno',
          etiqueta: 'Tipo de turno',
          tipo: 'select',
          opciones: () => [
            ['continuo', 'Continuo'],
            ['turno', 'Por turno'],
          ],
        },
        { nombre: 'dias_trabajo', etiqueta: 'Días de trabajo', tipo: 'multiselect', opciones: () => pares(cat.data?.dias) },
        {
          nombre: 'departamento_region',
          etiqueta: 'Departamento / región',
          tipo: 'select',
          otro: true,
          opciones: () => pares([...DEPARTAMENTOS]),
        },
        { nombre: 'manual_funciones_codigo', etiqueta: 'Código de manual de funciones', tipo: 'texto' },
        // Los tres tiempos (capacitación, OJT, ajustada) se cargan por cargo, dentro de "Cargos de
        // este servicio". Este valor general es el que hoy usa la base (crear_requerimiento_con_snapshot)
        // para calcular el inicio de capacitación. TODO(negocio): pasar esa función al valor por cargo.
        {
          nombre: 'tiempo_capacitacion_dias',
          etiqueta: 'Tiempo de capacitación general (días)',
          tipo: 'select',
          otro: true,
          opciones: opcionesDias,
          ayuda: 'Lo usa hoy la base al crear requerimientos. Los tiempos por cargo van en cada cargo.',
        },
        { nombre: 'capacitacion_pagada', etiqueta: 'Capacitación pagada', tipo: 'switch' },
        { nombre: 'tiene_refrigerio', etiqueta: 'Tiene refrigerio', tipo: 'switch' },
        { nombre: 'activo', etiqueta: 'Activo', tipo: 'switch' },
      ]}
      inicial={(s) => ({
        codigo: s?.codigo ?? '',
        nombre: s?.nombre ?? '',
        centro_costo_id: s?.centro_costo_id ?? '',
        cliente_id: s?.cliente_id ?? '',
        proyecto_id: s?.proyecto_id ?? '',
        contrato_comercial_id: s?.contrato_comercial_id ?? '',
        tipo_contrato: s?.tipo_contrato ?? '',
        // Valores legados en minúscula ("presencial") se muestran con el nombre del catálogo.
        modalidad: s ? (cat.data?.modalidades.find((m) => m.toLowerCase() === s.modalidad.toLowerCase()) ?? s.modalidad) : '',
        tipo_turno: s?.tipo_turno ?? '',
        dias_trabajo: s?.dias_trabajo ?? [],
        departamento_region: s?.departamento_region ?? '',
        manual_funciones_codigo: s?.manual_funciones_codigo ?? '',
        tiempo_capacitacion_dias: s?.tiempo_capacitacion_dias?.toString() ?? '',
        capacitacion_pagada: s?.capacitacion_pagada ?? false,
        tiene_refrigerio: s?.tiene_refrigerio ?? false,
        activo: s?.activo ?? true,
      })}
      validar={(v) => {
        const t = texto(v, 'tiempo_capacitacion_dias')
        return t && !NUMERO_ENTERO.test(t) ? 'El tiempo de capacitación debe ser un número entero de días.' : undefined
      }}
      extraEdicion={(s) => <CargosServicio servicioId={s.id} esRrhh={esRrhh} />}
      aBody={(v: Valores) => ({
        codigo: textoONulo(v, 'codigo'),
        nombre: texto(v, 'nombre'),
        centro_costo_id: textoONulo(v, 'centro_costo_id'),
        contrato_comercial_id: textoONulo(v, 'contrato_comercial_id'),
        cliente_id: textoONulo(v, 'cliente_id'),
        proyecto_id: textoONulo(v, 'proyecto_id'),
        tipo_contrato: textoONulo(v, 'tipo_contrato'),
        modalidad: texto(v, 'modalidad'),
        tipo_turno: textoONulo(v, 'tipo_turno'),
        dias_trabajo: v.dias_trabajo,
        departamento_region: textoONulo(v, 'departamento_region'),
        manual_funciones_codigo: textoONulo(v, 'manual_funciones_codigo'),
        tiempo_capacitacion_dias: numeroONulo(v, 'tiempo_capacitacion_dias'),
        capacitacion_pagada: v.capacitacion_pagada,
        tiene_refrigerio: v.tiene_refrigerio,
        activo: v.activo,
      })}
    />
  )
}

// --- 4. Cargos -------------------------------------------------------------------------

const TIPO_CARGO: Record<string, string> = { operativo: 'Operativo', administrativo_staff: 'Administrativo (staff)' }

export function CargosSeccion({ esRrhh }: Permisos) {
  const q = useConfigLista<CargoCfg>('cargos')
  return (
    <CrudSeccion<CargoCfg>
      titulo="Cargos"
      descripcion="Catálogo compartido: un cargo no pertenece a un servicio."
      recurso="cargos"
      filas={q.data}
      cargando={q.isPending}
      error={q.error}
      permisos={todo(esRrhh)}
      idDe={(c) => c.id}
      nombreDe={(c) => c.nombre}
      columnas={[
        { titulo: 'Cargo', valor: (c) => <span className="font-medium text-nova-titulo">{c.nombre}</span> },
        { titulo: 'Tipo', valor: (c) => TIPO_CARGO[c.tipo] ?? c.tipo },
      ]}
      campos={[
        { nombre: 'nombre', etiqueta: 'Nombre', tipo: 'texto', requerido: true },
        { nombre: 'tipo', etiqueta: 'Tipo', tipo: 'select', requerido: true, opciones: () => Object.entries(TIPO_CARGO) },
      ]}
      inicial={(c) => ({ nombre: c?.nombre ?? '', tipo: c?.tipo ?? 'operativo' })}
      aBody={(v) => ({ nombre: texto(v, 'nombre'), tipo: texto(v, 'tipo') })}
    />
  )
}

// --- 6. Contratos ----------------------------------------------------------------------

export function Contratos({ esRrhh, abrirId }: ConApertura) {
  const q = useConfigLista<ContratoCfg>('contratos')
  const clientes = useConfigLista<ClienteCfg>('clientes')
  const conFin = (v: Valores) => texto(v, 'fecha_fin') !== ''
  const conDuracion = (v: Valores) => texto(v, 'duracion_valor') !== '' || texto(v, 'duracion_unidad') !== ''
  return (
    <CrudSeccion<ContratoCfg>
      titulo="Contratos comerciales"
      descripcion="Vigencia por fecha de fin o por duración, nunca las dos."
      recurso="contratos"
      filas={q.data}
      cargando={q.isPending}
      error={q.error}
      permisos={todo(esRrhh)}
      idDe={(k) => k.id}
      abrirId={abrirId}
      nombreDe={(k) => k.codigo}
      columnas={[
        { titulo: 'Código', valor: (k) => <span className="font-medium text-nova-titulo">{k.codigo}</span> },
        { titulo: 'Nombre', valor: (k) => vacio(k.nombre) },
        { titulo: 'Cliente', valor: (k) => vacio(k.cliente_nombre) },
        { titulo: 'Inicio', valor: (k) => formatFecha(k.fecha_inicio), clase: 'whitespace-nowrap' },
        {
          titulo: 'Vigencia',
          valor: (k) =>
            k.fecha_fin ? `hasta ${formatFecha(k.fecha_fin)}` : k.duracion_valor ? `${k.duracion_valor} ${k.duracion_unidad === 'dias' ? 'días' : (k.duracion_unidad ?? '')}` : '—',
          clase: 'whitespace-nowrap',
        },
        {
          titulo: 'Renovación',
          valor: (k) =>
            k.tiempo_renovacion_valor
              ? `${k.tiempo_renovacion_valor} ${k.tiempo_renovacion_unidad === 'dias' ? 'días' : (k.tiempo_renovacion_unidad ?? '')} antes`
              : '—',
          clase: 'whitespace-nowrap',
        },
      ]}
      campos={[
        { nombre: 'codigo', etiqueta: 'Código', tipo: 'texto', requerido: true },
        { nombre: 'nombre', etiqueta: 'Nombre', tipo: 'texto' },
        { nombre: 'cliente_id', etiqueta: 'Cliente', tipo: 'select', opciones: () => (clientes.data ?? []).map((c) => [c.id, c.nombre]) },
        { nombre: 'fecha_inicio', etiqueta: 'Fecha de inicio', tipo: 'fecha', requerido: true },
        { nombre: 'fecha_fin', etiqueta: 'Fecha de fin', tipo: 'fecha', deshabilitado: conDuracion, ayuda: 'O bien una duración (se deshabilita si cargás duración).' },
        { nombre: 'duracion_valor', etiqueta: 'Duración', tipo: 'numero', deshabilitado: conFin },
        {
          nombre: 'duracion_unidad',
          etiqueta: 'Unidad de duración',
          tipo: 'select',
          deshabilitado: conFin,
          opciones: () => [
            ['meses', 'Meses'],
            ['dias', 'Días'],
          ],
        },
        {
          nombre: 'tiempo_renovacion_valor',
          etiqueta: 'Tiempo de renovación',
          tipo: 'numero',
          ayuda: 'Opcional. Cuánto antes del vencimiento empezar a renegociar.',
        },
        {
          nombre: 'tiempo_renovacion_unidad',
          etiqueta: 'Unidad de renovación',
          tipo: 'select',
          opciones: () => [
            ['meses', 'Meses'],
            ['dias', 'Días'],
          ],
        },
      ]}
      inicial={(k) => ({
        codigo: k?.codigo ?? '',
        nombre: k?.nombre ?? '',
        cliente_id: k?.cliente_id ?? '',
        fecha_inicio: k?.fecha_inicio ?? '',
        fecha_fin: k?.fecha_fin ?? '',
        duracion_valor: k?.duracion_valor?.toString() ?? '',
        duracion_unidad: k?.duracion_unidad ?? '',
        tiempo_renovacion_valor: k?.tiempo_renovacion_valor?.toString() ?? '',
        tiempo_renovacion_unidad: k?.tiempo_renovacion_unidad ?? '',
      })}
      validar={(v) => {
        if (conFin(v) && conDuracion(v)) return 'Elegí fecha de fin o duración, no ambas.'
        if (!conFin(v)) {
          if (!NUMERO_ENTERO.test(texto(v, 'duracion_valor')) || Number(texto(v, 'duracion_valor')) <= 0)
            return 'Indicá la fecha de fin o una duración (número entero positivo).'
          if (!texto(v, 'duracion_unidad')) return 'Elegí la unidad de duración.'
        }
        const rv = texto(v, 'tiempo_renovacion_valor')
        const ru = texto(v, 'tiempo_renovacion_unidad')
        if (rv || ru) {
          if (!NUMERO_ENTERO.test(rv) || Number(rv) <= 0) return 'Tiempo de renovación: número entero positivo (o dejalo vacío).'
          if (!ru) return 'Elegí la unidad del tiempo de renovación.'
        }
        return undefined
      }}
      aBody={(v) => ({
        codigo: texto(v, 'codigo'),
        nombre: textoONulo(v, 'nombre'),
        cliente_id: textoONulo(v, 'cliente_id'),
        fecha_inicio: textoONulo(v, 'fecha_inicio'),
        fecha_fin: textoONulo(v, 'fecha_fin'),
        duracion_valor: numeroONulo(v, 'duracion_valor'),
        duracion_unidad: textoONulo(v, 'duracion_unidad'),
        tiempo_renovacion_valor: numeroONulo(v, 'tiempo_renovacion_valor'),
        tiempo_renovacion_unidad: textoONulo(v, 'tiempo_renovacion_unidad'),
      })}
    />
  )
}

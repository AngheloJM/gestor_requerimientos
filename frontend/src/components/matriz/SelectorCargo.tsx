import { Plus } from 'lucide-react'
import { useState } from 'react'

import { type CargoCfg, useConfigGuardar } from '@/api'
import { ErrorApi } from '@/components/ErrorApi'
import { Button } from '@/components/ui/button'

export const TIPO_CARGO: Record<string, string> = { operativo: 'Operativo', administrativo_staff: 'Administrativo (staff)' }

const NUEVO = '__nuevo__'
const normalizar = (s: string) => s.trim().replace(/\s+/g, ' ').toLocaleLowerCase('es')

/**
 * Desplegable de cargos con la opción "+ Crear cargo nuevo" (revisión 06/11/2026, punto 4):
 * da de alta el cargo en el catálogo general (`cargos`) sin salir de la pantalla y lo deja
 * elegido. Se usa en "Agregar cargo" del servicio y en "Composición de cargo compartido".
 */
export function SelectorCargo({
  value,
  onChange,
  cargos,
  todos,
  disabled,
  className,
}: {
  value: string
  onChange: (id: string) => void
  /** Cargos que se ofrecen en la lista. */
  cargos: CargoCfg[]
  /** Catálogo completo, para avisar si el nombre ya existe (aunque no esté en la lista). */
  todos: CargoCfg[]
  disabled?: boolean
  className: string
}) {
  const guardar = useConfigGuardar('cargos')
  const [creando, setCreando] = useState(false)
  const [nombre, setNombre] = useState('')
  const [tipo, setTipo] = useState('operativo')
  const [aviso, setAviso] = useState<string>()

  const abrir = () => {
    setNombre('')
    setTipo('operativo')
    setAviso(undefined)
    guardar.reset()
    setCreando(true)
  }

  const crear = () => {
    const n = nombre.trim().replace(/\s+/g, ' ')
    if (n.length < 2) return setAviso('Indique el nombre del cargo.')
    const igual = todos.find((c) => normalizar(c.nombre) === normalizar(n))
    if (igual) {
      return setAviso(`Ya existe un cargo llamado “${igual.nombre}” (${TIPO_CARGO[igual.tipo] ?? igual.tipo}): elíjalo de la lista.`)
    }
    setAviso(undefined)
    guardar.mutate(
      { body: { nombre: n, tipo } },
      {
        onSuccess: (r) => {
          setCreando(false)
          onChange(r.id)
        },
      },
    )
  }

  return (
    <div className="grid gap-2">
      <select
        className={className}
        value={creando ? NUEVO : value}
        disabled={disabled}
        onChange={(e) => (e.target.value === NUEVO ? abrir() : (setCreando(false), onChange(e.target.value)))}
      >
        <option value="">Seleccione una opción</option>
        {cargos.map((c) => (
          <option key={c.id} value={c.id}>
            {c.nombre}
          </option>
        ))}
        <option value={NUEVO}>+ Crear cargo nuevo</option>
      </select>

      {creando && (
        // No es un <form>: suele ir dentro de otro formulario.
        <div className="grid gap-2 rounded-lg border border-dashed border-nova-azul/40 bg-nova-hielo/30 p-3" role="group" aria-label="Crear cargo nuevo">
          <label className="grid gap-1 text-xs text-nova-gris">
            Nombre del cargo nuevo *
            <input
              className={className}
              maxLength={120}
              value={nombre}
              onChange={(e) => setNombre(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  e.preventDefault()
                  crear()
                }
              }}
            />
          </label>
          <label className="grid gap-1 text-xs text-nova-gris">
            Tipo *
            <select className={className} value={tipo} onChange={(e) => setTipo(e.target.value)}>
              {Object.entries(TIPO_CARGO).map(([v, t]) => (
                <option key={v} value={v}>
                  {t}
                </option>
              ))}
            </select>
          </label>
          {aviso && (
            <p className="text-xs text-error" role="alert">
              {aviso}
            </p>
          )}
          {guardar.error ? <ErrorApi error={guardar.error} /> : null}
          <div className="flex justify-end gap-2">
            <Button type="button" variant="outline" size="sm" onClick={() => setCreando(false)} disabled={guardar.isPending}>
              Cancelar
            </Button>
            <Button type="button" size="sm" onClick={crear} disabled={guardar.isPending}>
              <Plus aria-hidden /> {guardar.isPending ? 'Creando…' : 'Crear cargo'}
            </Button>
          </div>
        </div>
      )}
    </div>
  )
}

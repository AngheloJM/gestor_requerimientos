import { CalendarOff, ChevronLeft, ChevronRight, Plus, Trash2 } from 'lucide-react'
import { useState } from 'react'

import { useCrearFeriado, useEliminarFeriado, useFeriados } from '@/api'
import { ErrorApi } from '@/components/ErrorApi'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import { formatFecha } from '@/lib/format'

function NuevoFeriado({ anio }: { anio: number }) {
  const crear = useCrearFeriado()
  const [fecha, setFecha] = useState('')
  const [nombre, setNombre] = useState('')
  const [aviso, setAviso] = useState<string>()

  const enviar = (e: React.FormEvent) => {
    e.preventDefault()
    const msg = !fecha ? 'Indicá la fecha.' : nombre.trim().length < 2 ? 'Indicá el nombre del feriado.' : undefined
    setAviso(msg)
    if (msg) return
    crear.mutate(
      { fecha, nombre: nombre.trim() },
      {
        onSuccess: () => {
          setFecha('')
          setNombre('')
        },
      },
    )
  }

  return (
    <form onSubmit={enviar} className="grid gap-3 border-b p-4 sm:grid-cols-[11rem_1fr_auto] sm:items-end">
      <div className="grid gap-1.5">
        <Label htmlFor="feriado-fecha">Fecha</Label>
        <Input id="feriado-fecha" type="date" min={`${anio}-01-01`} max={`${anio}-12-31`} className="h-9 bg-white" value={fecha} onChange={(e) => setFecha(e.target.value)} />
      </div>
      <div className="grid gap-1.5">
        <Label htmlFor="feriado-nombre">Nombre</Label>
        <Input id="feriado-nombre" maxLength={120} className="h-9 bg-white" value={nombre} onChange={(e) => setNombre(e.target.value)} />
      </div>
      <Button type="submit" disabled={crear.isPending}>
        <Plus aria-hidden /> {crear.isPending ? 'Agregando…' : 'Agregar feriado'}
      </Button>
      {aviso && (
        <p className="text-sm text-error sm:col-span-3" role="alert">
          {aviso}
        </p>
      )}
      {crear.error ? (
        <div className="sm:col-span-3">
          <ErrorApi error={crear.error} />
        </div>
      ) : null}
    </form>
  )
}

/**
 * Días feriados del año (revisión 06/11/2026, punto 12): se descuentan, junto con sábados y
 * domingos, del tiempo de reclutamiento. Por ahora solo feriados nacionales.
 */
export function Feriados({ esRrhh }: { esRrhh: boolean }) {
  const [anio, setAnio] = useState(() => new Date().getFullYear())
  const { data, isPending, error } = useFeriados(anio)
  const eliminar = useEliminarFeriado()

  return (
    <section className="rounded-xl border bg-card shadow-nova-sm" aria-label="Días feriados">
      <div className="flex flex-wrap items-center justify-between gap-3 border-b px-4 py-3">
        <div>
          <h3 className="flex items-center gap-2 font-semibold text-nova-titulo">
            <CalendarOff className="size-4 text-nova-azul" aria-hidden /> Días feriados
          </h3>
          <p className="text-xs text-nova-gris">Feriados nacionales. Se descuentan, junto con sábados y domingos, del tiempo de reclutamiento.</p>
        </div>
        <div className="flex items-center gap-1">
          <Button variant="ghost" size="icon" onClick={() => setAnio(anio - 1)} aria-label="Año anterior">
            <ChevronLeft />
          </Button>
          <span className="w-12 text-center font-medium tabular-nums text-nova-titulo">{anio}</span>
          <Button variant="ghost" size="icon" onClick={() => setAnio(anio + 1)} aria-label="Año siguiente">
            <ChevronRight />
          </Button>
        </div>
      </div>

      {esRrhh && <NuevoFeriado anio={anio} />}
      {eliminar.error ? (
        <div className="p-4">
          <ErrorApi error={eliminar.error} />
        </div>
      ) : null}

      {error ? (
        <div className="p-4">
          <ErrorApi error={error} />
        </div>
      ) : isPending ? (
        <Skeleton className="m-4 h-24 rounded-lg" />
      ) : data.length === 0 ? (
        <p className="px-4 py-6 text-center text-sm text-nova-gris">No hay feriados cargados para {anio}.</p>
      ) : (
        <ul className="divide-y">
          {data.map((f) => (
            <li key={f.id} className="flex items-center gap-3 px-4 py-2.5 text-sm">
              <span className="w-24 tabular-nums text-nova-texto-2">{formatFecha(f.fecha)}</span>
              <span className="flex-1 text-nova-titulo">{f.nombre}</span>
              {esRrhh && (
                <Button variant="ghost" size="icon" onClick={() => eliminar.mutate(f.id)} disabled={eliminar.isPending} aria-label={`Eliminar ${f.nombre}`}>
                  <Trash2 className="text-error" />
                </Button>
              )}
            </li>
          ))}
        </ul>
      )}
    </section>
  )
}

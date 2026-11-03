import { CheckCircle2, Plus, SlidersHorizontal } from 'lucide-react'
import { useState } from 'react'

import { useMe, useParametros, useRegistrarParametro } from '@/api'
import { EstadoBadge } from '@/components/EstadoBadge'
import { ErrorApi } from '@/components/ErrorApi'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import { formatFecha } from '@/lib/format'
import { formatParametro, nombreParametro } from '@/lib/parametros'

const NUMERO = /^\d{1,9}(\.\d{1,4})?$/

function NuevoValor({ parametros }: { parametros: string[] }) {
  const registrar = useRegistrarParametro()
  const [parametro, setParametro] = useState(parametros[0] ?? '')
  const [valor, setValor] = useState('')
  const [desde, setDesde] = useState('')
  const [error, setError] = useState<string>()
  const [ok, setOk] = useState(false)

  const enviar = (e: React.FormEvent) => {
    e.preventDefault()
    setOk(false)
    if (!NUMERO.test(valor)) return setError('Indicá un número (punto como separador decimal).')
    if (!desde) return setError('Indicá desde qué fecha rige.')
    setError(undefined)
    registrar.mutate(
      { parametro, valor, vigente_desde: desde },
      {
        onSuccess: () => {
          setValor('')
          setDesde('')
          setOk(true)
        },
      },
    )
  }

  return (
    <form onSubmit={enviar} className="grid gap-3 rounded-xl border bg-card p-5 shadow-nova-sm sm:grid-cols-[1fr_10rem_11rem_auto] sm:items-end">
      <div className="grid gap-1.5">
        <Label htmlFor="parametro">Parámetro</Label>
        <select
          id="parametro"
          value={parametro}
          onChange={(e) => setParametro(e.target.value)}
          className="h-9 rounded-lg border border-input bg-white px-2.5 text-sm"
        >
          {parametros.map((p) => (
            <option key={p} value={p}>
              {nombreParametro(p)}
            </option>
          ))}
        </select>
      </div>
      <div className="grid gap-1.5">
        <Label htmlFor="valor">Nuevo valor</Label>
        <Input id="valor" inputMode="decimal" className="h-9 bg-white tabular-nums" value={valor} onChange={(e) => setValor(e.target.value)} placeholder={parametro.endsWith('_pct') ? '0.25' : '3600'} />
      </div>
      <div className="grid gap-1.5">
        <Label htmlFor="desde">Vigente desde</Label>
        <Input id="desde" type="date" className="h-9 bg-white" value={desde} onChange={(e) => setDesde(e.target.value)} />
      </div>
      <Button type="submit" disabled={registrar.isPending}>
        <Plus aria-hidden /> Registrar
      </Button>
      <div className="sm:col-span-4">
        {error && (
          <p className="text-xs text-error" role="alert">
            {error}
          </p>
        )}
        {registrar.error && <ErrorApi error={registrar.error} />}
        {ok && (
          <p className="flex items-center gap-1.5 text-sm text-exito" role="status">
            <CheckCircle2 className="size-4" aria-hidden /> Registrado. El valor anterior queda en el historial.
          </p>
        )}
        <p className="mt-1 text-xs text-nova-gris">
          Los porcentajes se cargan como fracción (0.25 = 25 %). Cada cambio agrega una vigencia nueva; no se pisa el valor anterior.
        </p>
      </div>
    </form>
  )
}

export function ParametrosPage() {
  const { data, isPending, error } = useParametros()
  const { data: me } = useMe()
  const esRrhh = me?.rol === 'rrhh_admin'

  if (error) return <ErrorApi error={error} />
  if (isPending) return <Skeleton className="h-64 rounded-xl" />

  const claves = [...new Set(data.historial.map((v) => v.parametro))]

  return (
    <div className="grid gap-5">
      <div>
        <h2 className="text-2xl font-semibold text-nova-titulo">Parámetros salariales</h2>
        <p className="mt-1 text-sm text-nova-gris">Valores globales vigentes al {formatFecha(data.fecha)}, con su historial.</p>
      </div>

      <ul className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
        {data.vigentes.map((v) => (
          <li key={v.parametro} className="rounded-xl border bg-card p-5 shadow-nova-sm">
            <div className="flex items-center gap-3">
              <span className="grid size-10 place-items-center rounded-lg bg-nova-hielo text-nova-azul">
                <SlidersHorizontal className="size-5" aria-hidden />
              </span>
              <p className="text-sm text-nova-gris">{nombreParametro(v.parametro)}</p>
            </div>
            <p className="mt-4 text-3xl font-semibold tabular-nums text-nova-titulo">{formatParametro(v.parametro, v.valor)}</p>
            <p className="mt-1 text-xs text-nova-gris">Vigente desde {formatFecha(v.vigente_desde)}</p>
          </li>
        ))}
      </ul>

      {esRrhh && claves.length > 0 && <NuevoValor parametros={claves} />}

      <section className="overflow-x-auto rounded-xl border bg-card shadow-nova-sm">
        <h3 className="border-b px-4 py-3 font-semibold text-nova-titulo">Historial</h3>
        <table className="w-full min-w-[520px] text-sm">
          <thead className="bg-nova-hielo/60 text-left text-xs tracking-wide text-nova-gris uppercase">
            <tr>
              {['Parámetro', 'Valor', 'Vigente desde', ''].map((h) => (
                <th key={h} className="px-4 py-2.5 font-medium">
                  {h}
                </th>
              ))}
            </tr>
          </thead>
          <tbody className="divide-y">
            {data.historial.map((v) => {
              const vigente = data.vigentes.some((x) => x.id === v.id)
              const futuro = v.vigente_desde > data.fecha
              return (
                <tr key={v.id}>
                  <td className="px-4 py-2.5">{nombreParametro(v.parametro)}</td>
                  <td className="px-4 py-2.5 tabular-nums">{formatParametro(v.parametro, v.valor)}</td>
                  <td className="px-4 py-2.5">{formatFecha(v.vigente_desde)}</td>
                  <td className="px-4 py-2.5">
                    {vigente && <EstadoBadge tono="exito">Vigente</EstadoBadge>}
                    {futuro && <EstadoBadge tono="info">Programado</EstadoBadge>}
                  </td>
                </tr>
              )
            })}
          </tbody>
        </table>
      </section>
    </div>
  )
}

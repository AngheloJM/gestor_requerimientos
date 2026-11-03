import { type Area, useTablero } from '@/api'
import { ErrorApi } from '@/components/ErrorApi'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Skeleton } from '@/components/ui/skeleton'

const COLUMNAS: { key: Area; titulo: string }[] = [
  { key: 'comercial', titulo: 'Comercial' },
  { key: 'operaciones', titulo: 'Operaciones' },
  { key: 'rrhh', titulo: 'RR.HH.' },
]

export function TableroPage() {
  const { data, isPending, error } = useTablero()

  if (isPending) return <Skeleton className="h-32 w-full" />
  if (error) return <ErrorApi error={error} />

  const { indicadores: i, kanban, criterios_sin_definir } = data
  const kpis = [
    { titulo: 'Servicios configurados', valor: i.servicios_configurados, de: i.servicios_total },
    { titulo: 'Servicios incompletos', valor: i.servicios_incompletos, de: i.servicios_total },
    { titulo: 'Áreas sin completar tarea', valor: i.areas_sin_completar, de: 3 },
  ]

  return (
    <div className="grid gap-6">
      <div className="grid gap-4 sm:grid-cols-3">
        {kpis.map((k) => (
          <Card key={k.titulo}>
            <CardHeader>
              <CardTitle className="text-sm font-normal text-muted-foreground">{k.titulo}</CardTitle>
            </CardHeader>
            <CardContent className="text-3xl font-semibold tabular-nums">
              {k.valor}
              <span className="text-base font-normal text-muted-foreground"> / {k.de}</span>
            </CardContent>
          </Card>
        ))}
      </div>
      {criterios_sin_definir.length > 0 && (
        <p className="text-xs text-muted-foreground">
          Criterios aún no evaluados: {criterios_sin_definir.join(', ')}.
        </p>
      )}

      <h2 className="text-lg font-semibold">Solicitudes de cambio</h2>
      <div className="grid gap-4 md:grid-cols-3">
        {COLUMNAS.map(({ key, titulo }) => (
          <Card key={key}>
            <CardHeader>
              <CardTitle>
                {titulo} <span className="text-muted-foreground">({kanban[key].length})</span>
              </CardTitle>
            </CardHeader>
            <CardContent>
              {kanban[key].length === 0 ? (
                <p className="text-sm text-muted-foreground">Sin solicitudes</p>
              ) : (
                <ul className="grid gap-3">
                  {kanban[key].map((t) => (
                    <li key={t.id} className="rounded-md border p-3 text-sm">
                      <div className="flex items-center justify-between gap-2">
                        <span className="font-medium">{t.servicio.nombre ?? 'Servicio'}</span>
                        {t.estado && <Badge variant="secondary">{t.estado}</Badge>}
                      </div>
                      <div className="text-muted-foreground">
                        {t.campo}
                        {t.cargo?.nombre ? ` · ${t.cargo.nombre}` : ''}
                      </div>
                      {t.motivo && <div className="mt-1">{t.motivo}</div>}
                    </li>
                  ))}
                </ul>
              )}
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  )
}

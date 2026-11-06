import { UserRound } from 'lucide-react'

export interface DatosSolicitante {
  nombre: string | null | undefined
  cargo: string | null | undefined
  division: string | null | undefined
  region: string | null | undefined
  inmediato_superior: string | null | undefined
}

/** Datos fijos del solicitante (columna derecha, junto a los datos del servicio). */
export function PanelSolicitante({ datos }: { datos: DatosSolicitante }) {
  const filas: [string, string | null | undefined][] = [
    ['Nombre', datos.nombre],
    ['Cargo', datos.cargo],
    ['División', datos.division],
    ['Región', datos.region],
    ['Inmediato superior', datos.inmediato_superior ?? 'No asignado'],
  ]
  return (
    <aside className="rounded-xl border bg-card shadow-nova-sm" aria-label="Datos del solicitante">
      <div className="flex items-center gap-2 px-5 py-4">
        <UserRound className="size-4 text-nova-azul" aria-hidden />
        <h3 className="font-semibold text-nova-titulo">Datos del solicitante</h3>
      </div>
      <dl className="grid gap-3 border-t px-5 py-4">
        {filas.map(([t, v]) => (
          <div key={t}>
            <dt className="text-xs text-nova-gris">{t}</dt>
            <dd className="mt-0.5 text-sm break-words text-nova-titulo">{v || '—'}</dd>
          </div>
        ))}
      </dl>
    </aside>
  )
}

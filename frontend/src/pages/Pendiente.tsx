import { Construction } from 'lucide-react'

/** Estado vacío para pantallas aún no construidas (los datos ya están en `@/api`). */
export function Pendiente({ titulo, hook }: { titulo: string; hook: string }) {
  return (
    <div className="grid place-items-center rounded-xl border border-dashed bg-card px-6 py-16 text-center shadow-nova-xs">
      <span className="grid size-12 place-items-center rounded-xl bg-nova-naranja-claro text-nova-naranja">
        <Construction className="size-6" aria-hidden />
      </span>
      <h2 className="mt-4 text-lg font-semibold">{titulo}</h2>
      <p className="mt-1 max-w-md text-sm text-nova-gris">
        Pantalla en construcción. Los datos ya están disponibles en la API mediante <code>{hook}</code>.
      </p>
    </div>
  )
}

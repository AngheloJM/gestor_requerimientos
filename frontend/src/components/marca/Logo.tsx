import { cn } from '@/lib/utils'

/**
 * Logotipo NOVA (versión tipográfica).
 * TODO: reemplazar por el SVG oficial del isotipo cuando esté disponible.
 */
export function Logo({ claro = false, className }: { claro?: boolean; className?: string }) {
  return (
    <div className={cn('flex items-center gap-2.5', className)}>
      <span
        aria-hidden
        className="grid size-9 place-items-center rounded-lg bg-gradient-to-br from-nova-naranja to-nova-naranja-suave text-lg font-bold text-white shadow-nova-sm"
      >
        N
      </span>
      <div className="leading-none">
        <span className={cn('block text-xl font-bold tracking-wide', claro ? 'text-white' : 'text-nova-navy')}>
          NOVA
        </span>
        <span className={cn('block text-[11px]', claro ? 'text-white/70' : 'text-nova-gris')}>
          Gestor de Requerimientos
        </span>
      </div>
    </div>
  )
}

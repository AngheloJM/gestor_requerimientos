import { cn } from '@/lib/utils'

export type Tono = 'exito' | 'advertencia' | 'error' | 'info' | 'neutral' | 'marca'

const TONOS: Record<Tono, string> = {
  exito: 'border-exito/30 bg-exito/10 text-exito',
  advertencia: 'border-advertencia/40 bg-advertencia/10 text-[#B45309]',
  error: 'border-error/30 bg-error/10 text-error',
  info: 'border-info/30 bg-info/10 text-info',
  neutral: 'border-border bg-white text-nova-texto-2',
  marca: 'border-nova-naranja/30 bg-nova-naranja-claro text-[#B85F00]',
}

/** Badge con los colores de estado NOVA (pastilla con borde, como en la guía). */
export function EstadoBadge({
  tono = 'neutral',
  children,
  className,
}: {
  tono?: Tono
  children: React.ReactNode
  className?: string
}) {
  return (
    <span
      className={cn(
        'inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-medium whitespace-nowrap',
        TONOS[tono],
        className,
      )}
    >
      {children}
    </span>
  )
}

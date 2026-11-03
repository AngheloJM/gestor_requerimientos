import { cn } from '@/lib/utils'

/** Interruptor accesible (role="switch") para valores sí/no. */
export function Switch({
  id,
  checked,
  onChange,
  disabled,
  etiqueta,
}: {
  id?: string
  checked: boolean
  onChange: (v: boolean) => void
  disabled?: boolean
  /** Texto accesible si no hay <label> asociado. */
  etiqueta?: string
}) {
  return (
    <button
      id={id}
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={etiqueta}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={cn(
        'relative inline-flex h-6 w-11 shrink-0 items-center rounded-full border-2 border-transparent transition-colors outline-none focus-visible:ring-3 focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50',
        checked ? 'bg-nova-azul' : 'bg-nova-gris/30',
      )}
    >
      <span
        aria-hidden
        className={cn('inline-block size-5 rounded-full bg-white shadow-nova-sm transition-transform', checked ? 'translate-x-5' : 'translate-x-0')}
      />
    </button>
  )
}

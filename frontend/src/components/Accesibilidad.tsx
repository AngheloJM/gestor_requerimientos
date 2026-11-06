import { Contrast } from 'lucide-react'
import { useState } from 'react'

import { type Preferencias, aplicarPreferencias, leerPreferencias } from '@/lib/accesibilidad'
import { cn } from '@/lib/utils'

/** Botón de alto contraste (en la barra superior). */
export function Accesibilidad() {
  const [p, setP] = useState<Preferencias>(leerPreferencias)
  const cambiar = (nuevo: Preferencias) => {
    setP(nuevo)
    aplicarPreferencias(nuevo)
  }
  return (
    <button
      type="button"
      aria-pressed={p.contraste}
      onClick={() => cambiar({ contraste: !p.contraste })}
      className={cn(
        'ml-auto flex h-9 items-center gap-1.5 rounded-lg border bg-white px-2.5 text-sm font-medium text-nova-titulo transition-colors hover:bg-muted focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none',
        p.contraste && 'border-nova-titulo bg-nova-titulo text-white hover:bg-nova-titulo',
      )}
    >
      <Contrast className="size-4" aria-hidden />
      <span className="hidden sm:inline">Alto contraste</span>
    </button>
  )
}

import { Contrast, Type } from 'lucide-react'
import { useState } from 'react'

import { ESCALAS, type Preferencias, aplicarPreferencias, leerPreferencias } from '@/lib/accesibilidad'
import { cn } from '@/lib/utils'

/** Controles de tamaño de texto y alto contraste (en la barra superior). */
export function Accesibilidad() {
  const [p, setP] = useState<Preferencias>(leerPreferencias)
  const cambiar = (nuevo: Preferencias) => {
    setP(nuevo)
    aplicarPreferencias(nuevo)
  }
  return (
    <div className="ml-auto flex items-center gap-2" role="group" aria-label="Accesibilidad">
      <div className="flex items-center rounded-lg border bg-white p-0.5" role="radiogroup" aria-label="Tamaño del texto">
        <Type className="mx-1.5 size-4 text-nova-texto-2" aria-hidden />
        {ESCALAS.map((e, i) => (
          <button
            key={e.etiqueta}
            type="button"
            role="radio"
            aria-checked={p.escala === i}
            aria-label={`Texto ${e.etiqueta.toLowerCase()}`}
            title={`Texto ${e.etiqueta.toLowerCase()}`}
            onClick={() => cambiar({ ...p, escala: i })}
            className={cn(
              'grid h-8 min-w-8 place-items-center rounded-md px-1.5 font-semibold text-nova-titulo transition-colors hover:bg-muted focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none',
              ['text-xs', 'text-sm', 'text-base'][i],
              p.escala === i && 'bg-nova-azul text-white hover:bg-nova-azul',
            )}
          >
            A{'+'.repeat(i)}
          </button>
        ))}
      </div>
      <button
        type="button"
        aria-pressed={p.contraste}
        onClick={() => cambiar({ ...p, contraste: !p.contraste })}
        className={cn(
          'flex h-9 items-center gap-1.5 rounded-lg border bg-white px-2.5 text-sm font-medium text-nova-titulo transition-colors hover:bg-muted focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none',
          p.contraste && 'border-nova-titulo bg-nova-titulo text-white hover:bg-nova-titulo',
        )}
      >
        <Contrast className="size-4" aria-hidden />
        <span className="hidden sm:inline">Alto contraste</span>
      </button>
    </div>
  )
}

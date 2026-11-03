/** Placeholder para las pantallas que se portarán desde Lovable usando los hooks de `@/api`. */
export function Pendiente({ titulo, hook }: { titulo: string; hook: string }) {
  return (
    <div className="grid gap-2">
      <h1 className="text-xl font-semibold">{titulo}</h1>
      <p className="text-sm text-muted-foreground">
        Pendiente de portar desde Lovable. Datos disponibles vía <code>{hook}</code> en{' '}
        <code>@/api</code>.
      </p>
    </div>
  )
}

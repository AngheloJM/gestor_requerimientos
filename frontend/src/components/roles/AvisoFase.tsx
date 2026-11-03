import { Info } from 'lucide-react'

/** Aviso honesto de la fase actual: los permisos se registran pero todavía no se aplican. */
export function AvisoFase() {
  return (
    <div className="flex gap-3 rounded-xl border border-info/30 bg-info/5 p-4 text-sm text-nova-texto-2">
      <Info className="mt-0.5 size-4 shrink-0 text-info" aria-hidden />
      <p>
        <strong className="text-nova-titulo">Los permisos se guardan, pero todavía no cambian el acceso.</strong> Hoy cada
        usuario sigue entrando con uno de los 4 roles de sistema. Asignar roles nuevos a usuarios y aplicar estos permisos
        en la base es la siguiente fase, que se habilita con su confirmación.
      </p>
    </div>
  )
}

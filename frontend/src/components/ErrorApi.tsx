import { ApiError } from '@/api'
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'

const MENSAJES: Record<number, string> = {
  401: 'Tu sesión expiró. Volvé a iniciar sesión.',
  403: 'No tenés permiso para ver esta información.',
  404: 'No se encontró el recurso.',
}

/** Muestra un error de la API con un mensaje entendible según el status. */
export function ErrorApi({ error }: { error: unknown }) {
  const mensaje =
    error instanceof ApiError
      ? (MENSAJES[error.status] ?? error.detalle ?? error.codigo)
      : 'No se pudo conectar con el servidor.'
  return (
    <Alert variant="destructive">
      <AlertTitle>Error</AlertTitle>
      <AlertDescription>{mensaje}</AlertDescription>
    </Alert>
  )
}

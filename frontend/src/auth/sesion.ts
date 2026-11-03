/**
 * Sesión contra la API propia.
 * - El access token vive SOLO en memoria (no localStorage: no queda expuesto a XSS persistente).
 * - El refresh token está en una cookie HttpOnly que JavaScript no puede leer; se usa vía
 *   POST /auth/refresh (con `credentials: 'include'` y el encabezado anti-CSRF).
 */
import { ApiError } from '@/api/client'
import { env } from '@/lib/env'

export const HEADER_CSRF = { 'X-CSRF': '1' } as const

interface TokenRespuesta {
  access_token: string
  token_type: 'Bearer'
  expires_in: number
}

let accessToken: string | null = null
let refrescoEnCurso: Promise<string | null> | null = null
const oyentes = new Set<(autenticado: boolean) => void>()

export function getAccessToken(): string | null {
  return accessToken
}

function setAccessToken(t: string | null) {
  const antes = accessToken !== null
  accessToken = t
  if (antes !== (t !== null)) oyentes.forEach((f) => f(t !== null))
}

/** Suscripción a cambios de estado (ej. la sesión expiró y no se pudo renovar). */
export function alCambiarSesion(f: (autenticado: boolean) => void): () => void {
  oyentes.add(f)
  return () => oyentes.delete(f)
}

async function postAuth(ruta: string, body?: unknown): Promise<Response> {
  return fetch(`${env.apiUrl}${ruta}`, {
    method: 'POST',
    credentials: 'include',
    headers: {
      ...HEADER_CSRF,
      ...(body !== undefined ? { 'Content-Type': 'application/json' } : {}),
    },
    body: body !== undefined ? JSON.stringify(body) : undefined,
  })
}

async function errorDe(res: Response): Promise<ApiError> {
  const json = (await res.json().catch(() => null)) as { error?: string; detalle?: string } | null
  const retry = res.headers.get('Retry-After')
  return new ApiError(res.status, json?.error ?? `http_${res.status}`, json?.detalle ?? retry ?? undefined)
}

export async function login(email: string, password: string): Promise<void> {
  const res = await postAuth('/auth/login', { email, password })
  if (!res.ok) throw await errorDe(res)
  setAccessToken(((await res.json()) as TokenRespuesta).access_token)
}

/**
 * Renueva el access token con la cookie de refresh. Una sola petición a la vez
 * (varias llamadas concurrentes comparten la misma promesa: el backend rota el token).
 */
export function refrescar(): Promise<string | null> {
  refrescoEnCurso ??= (async () => {
    try {
      const res = await postAuth('/auth/refresh')
      const t = res.ok ? ((await res.json()) as TokenRespuesta).access_token : null
      setAccessToken(t)
      return t
    } catch {
      setAccessToken(null)
      return null
    } finally {
      refrescoEnCurso = null
    }
  })()
  return refrescoEnCurso
}

export async function logout(): Promise<void> {
  try {
    await postAuth('/auth/logout')
  } finally {
    setAccessToken(null)
  }
}

/**
 * Cliente HTTP para la API en Rust. Agrega el JWT de Supabase como Bearer y normaliza errores.
 */

/** Error de la API. `codigo` corresponde a `{"error": ...}` del backend. */
export class ApiError extends Error {
  readonly status: number
  readonly codigo: string
  readonly detalle?: string

  constructor(status: number, codigo: string, detalle?: string) {
    super(detalle ? `${codigo}: ${detalle}` : codigo)
    this.name = 'ApiError'
    this.status = status
    this.codigo = codigo
    this.detalle = detalle
  }
}

export type QueryParams = Record<string, string | number | null | undefined>

export interface RequestOptions {
  method?: 'GET' | 'POST' | 'PATCH' | 'PUT' | 'DELETE'
  query?: QueryParams
  body?: unknown
  signal?: AbortSignal
}

export interface ApiClientConfig {
  baseUrl: string
  getToken: () => Promise<string | null>
  /** Ante un 401, intenta renovar la sesión; si devuelve un token, reintenta una vez. */
  onUnauthorized?: () => Promise<string | null>
  /** Inyectable para tests. */
  fetch?: typeof fetch
}

export function buildUrl(baseUrl: string, path: string, query?: QueryParams): string {
  // Base relativa (ej. "/api" detrás del proxy de Vite) o absoluta.
  const url = new URL(baseUrl.replace(/\/+$/, '') + path, globalThis.location?.origin ?? 'http://localhost')
  for (const [k, v] of Object.entries(query ?? {})) {
    // null/undefined/'' = filtro no aplicado: no se envía.
    if (v !== null && v !== undefined && v !== '') url.searchParams.set(k, String(v))
  }
  return /^https?:\/\//.test(baseUrl) ? url.toString() : url.pathname + url.search
}

async function parseError(res: Response): Promise<ApiError> {
  const texto = await res.text().catch(() => '')
  try {
    const json = JSON.parse(texto) as { error?: string; detalle?: string }
    if (json && typeof json.error === 'string') {
      return new ApiError(res.status, json.error, json.detalle)
    }
  } catch {
    // Respuesta no JSON (ej. rechazos de extractores de Axum en texto plano).
  }
  return new ApiError(res.status, `http_${res.status}`, texto || res.statusText)
}

export function createApiClient(config: ApiClientConfig) {
  const doFetch = config.fetch ?? fetch.bind(globalThis)

  async function enviar(path: string, opts: RequestOptions, token: string | null) {
    const headers: Record<string, string> = { Accept: 'application/json', 'X-CSRF': '1' }
    if (token) headers.Authorization = `Bearer ${token}`
    if (opts.body !== undefined) headers['Content-Type'] = 'application/json'
    return doFetch(buildUrl(config.baseUrl, path, opts.query), {
      method: opts.method ?? 'GET',
      headers,
      credentials: 'include',
      body: opts.body !== undefined ? JSON.stringify(opts.body) : undefined,
      signal: opts.signal,
    })
  }

  async function request<T>(path: string, opts: RequestOptions = {}): Promise<T> {
    let res = await enviar(path, opts, await config.getToken())
    if (res.status === 401 && config.onUnauthorized) {
      const nuevo = await config.onUnauthorized()
      if (nuevo) res = await enviar(path, opts, nuevo)
    }

    if (!res.ok) throw await parseError(res)
    if (res.status === 204) return undefined as T
    return (await res.json()) as T
  }

  return {
    request,
    get: <T>(path: string, query?: QueryParams, signal?: AbortSignal) =>
      request<T>(path, { query, signal }),
    post: <T>(path: string, body: unknown) => request<T>(path, { method: 'POST', body }),
    patch: <T>(path: string, body: unknown) => request<T>(path, { method: 'PATCH', body }),
    put: <T>(path: string, body: unknown) => request<T>(path, { method: 'PUT', body }),
    del: <T>(path: string) => request<T>(path, { method: 'DELETE' }),
  }
}

export type ApiClient = ReturnType<typeof createApiClient>

import { describe, expect, it, vi } from 'vitest'

import { ApiError, buildUrl, createApiClient } from '@/api/client'

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  })
}

function cliente(fetchImpl: typeof fetch, token: string | null = 'tok-123') {
  return createApiClient({
    baseUrl: 'http://api.test/',
    getToken: async () => token,
    fetch: fetchImpl,
  })
}

describe('buildUrl', () => {
  it('omite parámetros vacíos y normaliza la barra final', () => {
    expect(
      buildUrl('http://api.test/', '/matriz-servicio', {
        cliente_id: 'abc',
        servicio_id: null,
        centro_costo_id: undefined,
        x: '',
      }),
    ).toBe('http://api.test/matriz-servicio?cliente_id=abc')
    expect(buildUrl('http://api.test', '/almanaque', { mes: 11, anio: 2026 })).toBe(
      'http://api.test/almanaque?mes=11&anio=2026',
    )
  })
})

describe('createApiClient', () => {
  it('envía el Bearer token y parsea JSON', async () => {
    const f = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(200, { user_id: 'u', rol: 'estandar' }))
    const r = await cliente(f).get<{ rol: string }>('/me')
    expect(r.rol).toBe('estandar')
    const [url, init] = f.mock.calls[0]!
    expect(url).toBe('http://api.test/me')
    expect((init!.headers as Record<string, string>).Authorization).toBe('Bearer tok-123')
  })

  it('sin sesión no envía Authorization', async () => {
    const f = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(200, {}))
    await cliente(f, null).get('/health')
    expect((f.mock.calls[0]![1]!.headers as Record<string, string>).Authorization).toBeUndefined()
  })

  it('un 201 sin cuerpo resuelve sin error', async () => {
    const f = vi.fn<typeof fetch>().mockResolvedValue(new Response(null, { status: 201 }))
    await expect(cliente(f).post('/feriados', { fecha: '2026-11-09', nombre: 'X' })).resolves.toBeUndefined()
  })

  it('PATCH serializa el body con Content-Type JSON', async () => {
    const f = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(200, { id: '1' }))
    await cliente(f).patch('/aprobaciones/1', { fecha_fin: null })
    const init = f.mock.calls[0][1]!
    expect(init.method).toBe('PATCH')
    expect(init.body).toBe('{"fecha_fin":null}')
    expect((init.headers as Record<string, string>)['Content-Type']).toBe('application/json')
  })

  it('convierte errores JSON del backend en ApiError', async () => {
    const f = vi.fn<typeof fetch>().mockResolvedValue(
      jsonResponse(422, { error: 'validation_error', detalle: 'la fecha de fin es anterior' }),
    )
    const err = await cliente(f).patch('/aprobaciones/1', { fecha_fin: '2020-01-01' }).catch((e) => e)
    expect(err).toBeInstanceOf(ApiError)
    expect(err).toMatchObject({ status: 422, codigo: 'validation_error', detalle: 'la fecha de fin es anterior' })
  })

  it('soporta errores en texto plano (rechazos de Axum)', async () => {
    const f = vi.fn<typeof fetch>().mockResolvedValue(new Response('Invalid URL: bad uuid', { status: 400 }))
    const err = await cliente(f).get('/aprobaciones/x').catch((e) => e)
    expect(err).toMatchObject({ status: 400, codigo: 'http_400', detalle: 'Invalid URL: bad uuid' })
  })
})

describe('renovación de sesión ante 401', () => {
  it('refresca una vez y reintenta con el token nuevo', async () => {
    const f = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(jsonResponse(401, { error: 'unauthorized' }))
      .mockResolvedValueOnce(jsonResponse(200, { ok: true }))
    const onUnauthorized = vi.fn().mockResolvedValue('tok-nuevo')
    const c = createApiClient({ baseUrl: 'http://api.test', getToken: async () => 'tok-viejo', fetch: f, onUnauthorized })
    expect(await c.get('/me')).toEqual({ ok: true })
    expect(onUnauthorized).toHaveBeenCalledTimes(1)
    expect((f.mock.calls[1]![1]!.headers as Record<string, string>).Authorization).toBe('Bearer tok-nuevo')
  })

  it('si no se puede refrescar, propaga el 401', async () => {
    const f = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(401, { error: 'unauthorized' }))
    const c = createApiClient({ baseUrl: 'http://api.test', getToken: async () => 't', fetch: f, onUnauthorized: async () => null })
    await expect(c.get('/me')).rejects.toMatchObject({ status: 401 })
    expect(f).toHaveBeenCalledTimes(1)
  })

  it('envía cookies y encabezado anti-CSRF; base relativa', async () => {
    const f = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(200, {}))
    const c = createApiClient({ baseUrl: '/api', getToken: async () => null, fetch: f })
    await c.get('/tablero', { x: 1 })
    expect(f.mock.calls[0]![0]).toBe('/api/tablero?x=1')
    expect(f.mock.calls[0]![1]!.credentials).toBe('include')
    expect((f.mock.calls[0]![1]!.headers as Record<string, string>)['X-CSRF']).toBe('1')
  })
})

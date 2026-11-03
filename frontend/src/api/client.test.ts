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

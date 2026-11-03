import { QueryClient } from '@tanstack/react-query'

import { ApiError } from '@/api/client'

/** Errores 4xx no se reintentan (permiso, validación, no encontrado): reintentar no ayuda. */
function shouldRetry(failureCount: number, error: unknown): boolean {
  if (error instanceof ApiError && error.status >= 400 && error.status < 500) return false
  return failureCount < 2
}

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: { staleTime: 30_000, retry: shouldRetry, refetchOnWindowFocus: false },
    mutations: { retry: false },
  },
})

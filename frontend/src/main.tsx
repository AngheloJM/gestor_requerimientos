import { QueryClientProvider } from '@tanstack/react-query'
import { ReactQueryDevtools } from '@tanstack/react-query-devtools'
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'

import './index.css'
import App from '@/App'
import { AuthProvider } from '@/auth/AuthProvider'
import { aplicarPreferencias, leerPreferencias } from '@/lib/accesibilidad'
import { queryClient } from '@/lib/query-client'

// Tamaño de texto y contraste elegidos por el usuario, antes del primer render.
aplicarPreferencias(leerPreferencias())

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <AuthProvider>
        <App />
      </AuthProvider>
      <ReactQueryDevtools initialIsOpen={false} />
    </QueryClientProvider>
  </StrictMode>,
)

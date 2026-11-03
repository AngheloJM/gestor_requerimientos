# Frontend — Gestor de Requerimientos

Vite + React + TypeScript + Tailwind v4 + shadcn/ui (base radix, preset nova) + TanStack Query.

## Arquitectura de datos

- **Datos**: siempre vía la API en Rust (`VITE_API_URL`), con los hooks de `@/api`
  (`useTablero`, `useMatrizServicio`, `useAlmanaque`, `useAprobaciones`, `useEditarFechaFin`, `useMe`).
- **Autenticación** contra la API propia (`src/auth/sesion.ts`): el access token vive solo en
  memoria; el refresh token es una cookie HttpOnly que el navegador envía a `/auth/refresh`.
  Ante un 401 el cliente renueva la sesión una vez y reintenta.
- **Montos**: llegan como string (`"3750.50"`). No convertir a `number` para calcular;
  usar `formatMonto` solo para mostrar.

## Puesta en marcha

```bash
npm install
npm run dev                  # http://localhost:5173
```

En desarrollo Vite redirige `/api/*` al backend (`127.0.0.1:8080`): mismo origen, sin CORS.

## Scripts

| Script | Qué hace |
|---|---|
| `npm run dev` | servidor de desarrollo |
| `npm run build` | typecheck + build de producción |
| `npm test` | tests (Vitest) |
| `npm run lint` | oxlint |

## Portar pantallas desde Lovable

Reemplazar cada `supabase.from('tabla').select(...)` por el hook correspondiente de `@/api`;
los componentes shadcn de Lovable (radix) se pueden copiar a `src/components/ui/`.

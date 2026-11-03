# Frontend — Gestor de Requerimientos

Vite + React + TypeScript + Tailwind v4 + shadcn/ui (base radix, preset nova) + TanStack Query.

## Arquitectura de datos

- **Datos**: siempre vía la API en Rust (`VITE_API_URL`), con los hooks de `@/api`
  (`useTablero`, `useMatrizServicio`, `useAlmanaque`, `useAprobaciones`, `useEditarFechaFin`, `useMe`).
- **Autenticación**: `@supabase/supabase-js` se usa **solo** para login/logout y obtener el
  `access_token`, que el cliente envía como `Authorization: Bearer` a la API.
  No usar `supabase.from(...)` / `supabase.rpc(...)`.
- **Montos**: llegan como string (`"3750.50"`). No convertir a `number` para calcular;
  usar `formatMonto` solo para mostrar.

## Puesta en marcha

```bash
cp .env.example .env.local   # completar; no versionar
npm install
npm run dev                  # http://localhost:5173
```

El backend debe tener `CORS_ALLOWED_ORIGINS=http://localhost:5173`.

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

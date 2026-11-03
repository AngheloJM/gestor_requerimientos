# Gestor de Requerimientos

Migración de la aplicación original (React + Supabase) a un backend propio en **Rust (Axum)**
y un frontend **React + TypeScript**, reutilizando la misma base Postgres de Supabase.

## Estructura

| Carpeta | Contenido |
|---|---|
| `backend/domain` | Entidades y reglas de negocio (sin HTTP ni base de datos) |
| `backend/db` | Consultas SQLx; cada request corre en una transacción con la identidad del usuario para que **RLS** autorice |
| `backend/api` | Servidor Axum: login propio (argon2id + JWT + refresh token rotativo), CORS y endpoints |
| `frontend` | Vite + React + TypeScript + Tailwind + shadcn/ui + TanStack Query |
| `infra/db` | Postgres local con Podman y la imagen `supabase/postgres` |
| `docs` | Pendientes y supuestos por resolver |

## Endpoints

`POST /auth/login` · `POST /auth/refresh` · `POST /auth/logout` · `POST /auth/cambiar-password` ·
`GET /me` · `GET /matriz-servicio` · `GET /almanaque?mes=&anio=` · `GET /tablero` ·
`GET /aprobaciones` · `PATCH /aprobaciones/{id}` · `GET /aprobaciones/{id}/historial`

## Seguridad

- Login propio: contraseñas con **argon2id**; access token JWT de 15 min (solo en memoria del
  navegador); refresh token opaco en cookie `HttpOnly`/`Secure`/`SameSite=Strict`, guardado como
  hash, rotado en cada uso y con detección de reuso; límite de intentos por cuenta e IP; error
  genérico y tiempo constante; encabezado anti-CSRF; encabezados de seguridad HTTP.
- La autorización la aplican las políticas RLS de Postgres; el backend propaga la identidad
  (`SET LOCAL ROLE authenticated` + `request.jwt.claims`).
- Alta/cambio de contraseña por consola: `cargo run -p api -- establecer-credenciales <usuario_id> <email>`.
- Las credenciales van en archivos `.env` locales (excluidos de git); ver los `.env.example`.

## Desarrollo local

```bash
# Base de datos
cd infra/db && ./db.sh init && ./db.sh up && ./db.sh cargar

# Backend (completar backend/.env a partir de .env.example)
cd backend && cargo run -p api

# Frontend (completar frontend/.env.local a partir de .env.example)
cd frontend && npm install && npm run dev
```

Tests: `cargo test` (unitarios) y `cargo test -p db -- --ignored` con `DATABASE_URL` de pruebas;
`npm test` en `frontend`.

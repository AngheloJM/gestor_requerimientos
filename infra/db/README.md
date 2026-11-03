# Base de datos local (Podman + imagen supabase/postgres)

Postgres con los mismos roles, schema `auth` y extensiones que Supabase, escuchando solo en
`127.0.0.1:54322`.

## Primera vez

```bash
sudo pacman -S podman podman-compose      # una sola vez
cd infra/db
./db.sh init      # crea .env con contraseña aleatoria (no versionar)
./db.sh up
./db.sh cargar    # schema real si hay archivos en schema/, si no fixtures aproximados
./db.sh url       # copiar el resultado a DATABASE_URL en backend/.env
```

## Qué tan parecido a prod es

| Igual que prod | Distinto |
|---|---|
| Imagen oficial de Supabase: roles `anon`/`authenticated`/`service_role`, `auth.uid()`, extensiones | Versión mayor de Postgres: verificar la de prod y ajustar `PG_IMAGE_TAG` |
| Se conecta como `postgres` (no superusuario), como en Supabase | Sin el dump real: tipos aproximados y **políticas RLS de prueba**, no las de prod |
| | No incluye Supabase Auth (login): los JWT se prueban con el JWKS simulado |
| | Sin datos reales (solo datos ficticios) |

#!/usr/bin/env bash
# Administra la base local (Podman). Uso:
#   ./db.sh init       genera .env con contraseña aleatoria (una sola vez)
#   ./db.sh up         levanta el contenedor y espera a que esté listo
#   ./db.sh cargar     carga el schema (schema/*.sql reales si existen, si no fixtures)
#                      y luego las migraciones de backend/migrations;
#   ./db.sh psql       consola psql dentro del contenedor
#   ./db.sh url        imprime el DATABASE_URL para backend/.env (contiene la contraseña local)
#   ./db.sh down       detiene el contenedor (conserva los datos)
#   ./db.sh reset      BORRA el volumen local y vuelve a crear la base vacía
set -euo pipefail
cd "$(dirname "$0")"
COMPOSE="podman-compose -f compose.yaml"
CONT=gestor-req-db
FIXTURES=../../backend/db/tests/fixtures

env_val() { grep -E "^$1=" .env | cut -d= -f2-; }

psql_in() { podman exec -i "$CONT" psql -v ON_ERROR_STOP=1 -U postgres -d postgres "$@"; }

esperar() {
  echo -n "esperando a Postgres"
  for _ in $(seq 1 60); do
    if podman exec "$CONT" pg_isready -U postgres -h localhost >/dev/null 2>&1; then echo " listo"; return; fi
    echo -n "."; sleep 1
  done
  echo; echo "Postgres no respondió a tiempo: revisar 'podman logs $CONT'" >&2; exit 1
}

case "${1:-}" in
  init)
    if [[ -f .env ]]; then echo ".env ya existe; no se sobrescribe"; exit 0; fi
    umask 077
    printf 'POSTGRES_PASSWORD=%s\nDB_PORT=54322\n# PG_IMAGE_TAG=17.11.0.004\n' \
      "$(head -c 24 /dev/urandom | base64 | tr -d '/+=')" > .env
    echo ".env creado (permisos 600). No versionar."
    ;;
  up)
    [[ -f .env ]] || { echo "falta .env: ejecutar './db.sh init'" >&2; exit 1; }
    $COMPOSE up -d
    esperar
    ;;
  cargar)
    shopt -s nullglob
    reales=(schema/*.sql)
    if (( ${#reales[@]} )); then
      echo "Cargando schema real (${#reales[@]} archivos de schema/):"
      for f in "${reales[@]}"; do echo "  $f"; psql_in -q < "$f"; done
    else
      echo "schema/ vacío → cargando fixtures APROXIMADOS (tipos y RLS de prueba, no los de prod)"
      for f in supabase_subset rls_test datos p2_edicion p2_creacion p2_almanaque p2_usuarios p2_parametros; do echo "  $f.sql"; psql_in -q < "$FIXTURES/$f.sql"; done
    fi
    # Migraciones propias (aditivas), en orden. 0001 es la baseline (solo comentarios hoy).
    for f in ../../backend/migrations/*.sql; do echo "  migración $(basename "$f")"; psql_in -q < "$f"; done
    psql_in -c "SELECT current_setting('server_version') AS version,
                       (SELECT count(*) FROM pg_tables WHERE schemaname='public') AS tablas_public"
    ;;
  psql) podman exec -it "$CONT" psql -U postgres -d postgres ;;
  url)  echo "postgres://postgres:$(env_val POSTGRES_PASSWORD)@127.0.0.1:$(env_val DB_PORT)/postgres" ;;
  down) $COMPOSE down ;;
  reset)
    read -r -p "Esto BORRA la base local (volumen gestor-req_gestor_req_db). Escribir 'si' para continuar: " ok
    [[ "$ok" == "si" ]] || { echo "cancelado"; exit 1; }
    $COMPOSE down -v
    $COMPOSE up -d
    esperar
    ;;
  *) sed -n '2,11p' "$0"; exit 1 ;;
esac

#!/usr/bin/env bash
# Carga (o vuelve a cargar) los datos de demostración en la base LOCAL.
# Solo usa la URL de la base local (./db.sh url); no acepta otra. Ver docs/DATOS_DE_PRUEBA.md.
set -euo pipefail
cd "$(dirname "$0")"

URL="$(./db.sh url)"
case "$URL" in
  *@127.0.0.1:*|*@localhost:*) ;;
  *) echo "La URL de ./db.sh no apunta a la base local; abortado." >&2; exit 1 ;;
esac

psql "$URL" -q -v ON_ERROR_STOP=1 -f seed_demo.sql

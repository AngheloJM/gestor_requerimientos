Colocar aquí el schema REAL de Supabase, en archivos `.sql` que se aplican en orden alfabético:

- las migraciones de Lovable (`supabase/migrations/*.sql` del repo original), o
- un dump: `supabase db dump --schema public -f 0001_schema.sql`
  (o `pg_dump --schema-only --schema=public --no-owner`).

Solo estructura, **sin datos de clientes**. Mientras esta carpeta no tenga `.sql`, `./db.sh cargar`
usa los fixtures aproximados de `backend/db/tests/fixtures/`.

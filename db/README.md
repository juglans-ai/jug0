# jug0 Database Management

## Directory Structure

```
db/
├── schema.sql          # Complete current schema (source of truth)
├── migrations/         # Incremental migration files
│   ├── 001_init.sql
│   ├── 002_add_username.sql
│   ├── 003_add_message_id.sql
│   ├── 004_replace_stateless_with_state.sql
│   ├── 005_add_handles.sql
│   ├── 006_add_persistent_handle_chat_index.sql
│   └── 007_add_external_id.sql
├── seeds/
│   └── dev_seed.sql    # Development seed data
└── README.md
```

## Prerequisites

Database must be initialized **before** starting the application. The Rust app does not run migrations at startup.

```bash
# Fresh database setup
psql -d jug0 -f db/schema.sql
psql -d jug0 -f db/seeds/dev_seed.sql

# Or run migrations incrementally
psql -d jug0 -f db/migrations/001_init.sql
psql -d jug0 -f db/migrations/002_add_username.sql
# ... etc
psql -d jug0 -f db/seeds/dev_seed.sql
```

## Conventions

### schema.sql

- Represents the **current** complete database structure (source of truth)
- Use for bootstrapping a fresh database
- Must be kept in sync with migrations — update after every new migration

### migrations/

- Files named `NNN_description.sql` (e.g. `008_add_new_table.sql`)
- Run manually via `psql -f` before deploying new code
- **Never modify** a migration that has been deployed — create a new one instead
- All migrations must be **idempotent** (`IF NOT EXISTS`, `ON CONFLICT DO NOTHING`)

### seeds/

- Development/test data with `ON CONFLICT DO NOTHING` for idempotency
- Run manually after migrations

## Workflow: Adding a Schema Change

1. Create a new migration file:
   ```
   db/migrations/008_add_feature_x.sql
   ```

2. Update `db/schema.sql` to reflect the new state

3. Test locally:
   ```bash
   # Fresh database from schema
   psql -d jug0_test -f db/schema.sql
   psql -d jug0_test -f db/seeds/dev_seed.sql

   # Or apply single migration to existing database
   psql -d jug0 -f db/migrations/008_add_feature_x.sql
   ```

4. Commit both the migration and updated schema.sql together

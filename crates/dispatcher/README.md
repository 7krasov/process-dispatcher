# process_dispatcher

The planner. Creates and tracks rows in `dispatcher_processes`, and hands them
out to supervisors over HTTP.

Binary: `process_dispatcher` (`src/bin/process_dispatcher.rs`).

## What it does

1. **Schedules new processes.** A background task wakes up, reads active rows
   from the external `sources` table, and for each source ensures there is at
   most one unfinished process — and at most one finished process per day.
   Missing rows are inserted with `state = Created`.
2. **Assigns processes to supervisors.** When a supervisor calls
   `GET /obtain_new_process/{supervisor_id}`, dispatcher picks an available
   process, marks it `Processing`, stamps the supervisor id, and returns
   `AssignedProcess`.
3. **Records terminal results.** When a supervisor calls
   `PATCH /report_process_finish/{process_id}`, dispatcher transitions the row
   to `Completed` or `Failed` depending on the reported result.

## Databases

Two MySQL connection pools, both configured via env vars:

| Pool | Env var | Schema | Ownership | Access |
|---|---|---|---|---|
| `pd` | `PD_DATABASE_URL` | `process_dispatcher` | this project | read/write |
| `mvp` | `MVP_DATABASE_URL` | `mvp` | external legacy project | **read only** (`sources`) |

Migrations live in:
- `db/migrations/` — the `dispatcher_processes` table (owned here).
- `db/mvp_migrations/` — a **local copy** of the external `sources` schema,
  kept only so local dev / docker-compose can bring the DB up. The upstream
  project is the real owner.

Run migrations locally via:

```bash
cd crates/dispatcher
make migrate        # process_dispatcher schema
make mvp.migrate    # mvp schema (dev copy)
```

### `dispatcher_processes` schema (essentials)

```
uuid          VARBINARY(16) PK        -- process id
source_id     INT UNSIGNED            -- FK into sources (logical)
supervisor_id VARBINARY(16) NULL      -- set on assign
state         VARCHAR(32)             -- DispatchState string
mode          VARCHAR(20)             -- ProcessingMode numeric as string
created_at    TIMESTAMP(3)
updated_at    TIMESTAMP(3)
```

Collation is `utf8mb4_bin`, which makes `sqlx` return string columns as `VARBINARY`.
`MySqlRowExt::get_string` in `src/dispatcher.rs` is the workaround.

## Scheduling logic

`Dispatcher::prepare_schedule` (see `src/dispatcher.rs`):

1. Stream `SELECT id FROM sources WHERE status = 'run'`.
2. For each `source_id`, take a per-source async mutex (`AsyncKeyedMutex`) so
   no two scheduler cycles race on the same source.
3. Look at the latest process for that source:
   - If it exists and is **not finished** — skip.
   - If it exists, is finished, and was created **today** — skip.
   - Otherwise insert a new row with `state = Created, mode = Regular`.

The main loop sleeps 60 s between cycles when the previous cycle produced zero
rows. When rows are produced it loops again immediately (noted as a gap — see below).

Timezone for the "today" check is hard-coded to `Europe/Berlin` via the
`TIMEZONE` constant.

## Assignment logic

`Dispatcher::assign_process`:

1. Stream sources that have assignable work:
   ```sql
   state IN (Created, Pending) AND supervisor_id IS NULL
   OR
   state = Error AND supervisor_id = :supervisor_id   -- retry by the same supervisor
   ORDER BY created_at ASC
   ```
2. For each candidate source, stream its oldest `Created`/`Pending` process,
   re-validate under the per-source lock, then `UPDATE` to
   `state = Processing, supervisor_id = :supervisor_id`.
3. Return the first successfully assigned row as `AssignedProcess`, or `None`.

## HTTP API

Exposed by `start_http_server` (`src/http_server.rs`) on `HTTP_PORT`
(default `8089`).

| Method | Path | Response |
|---|---|---|
| `GET` | `/obtain_new_process/{supervisor_id}` | `200` + `AssignedProcess` JSON, `204` if nothing, `500` on error. `supervisor_id` is a UUID. |
| `PATCH` | `/report_process_finish/{process_id}` | Body: `ProcessFinishReport`. `200` ok, `400` invalid `result`, `404` unknown uuid, `500` on DB error. |

Graceful shutdown: the HTTP server is wired to a `CancellationToken` that fires
on `SIGTERM` / `SIGINT` / `SIGQUIT`, draining in-flight connections via
`axum::serve(...).with_graceful_shutdown(...)`.

## Environment variables

| Var | Required | Default | Purpose |
|---|---|---|---|
| `HTTP_PORT` | no | `8089` | HTTP listen port. |
| `MAX_DB_CONNECTIONS` | no | `10` | Max pool size for **each** MySQL pool. |
| `PD_DATABASE_URL` | **yes** | — | `mysql://…/process_dispatcher` |
| `MVP_DATABASE_URL` | **yes** | — | `mysql://…/mvp` |
| `RUST_LOG` | no | `trace` | `tracing-subscriber` env filter. |

## Notable modules

| File | What's inside |
|---|---|
| `src/bin/process_dispatcher.rs` | Entrypoint: tracing init, env, signal handling, schedule loop spawn, HTTP server. |
| `src/dispatcher.rs` | `Dispatcher` struct, `prepare_schedule`, `assign_process`, `report_process_finish`, time helpers. |
| `src/db_repository.rs` | All raw `sqlx` queries. Everything the DB sees lives here. |
| `src/http_server.rs` + `src/http_server/route_handlers.rs` | axum router and handlers. |
| `src/async_keyed_mutex.rs` | Per-key tokio mutex registry with weak-ref cleanup — protects a single `source_id` across concurrent schedulers. |
| `src/cancellation_ext.rs` | Extension trait to wrap futures in `CancellationToken` without `tokio::select!` boilerplate. |
| `src/env.rs` | Env var parsing into `EnvParams`. |

## Known gaps / TODO

See the workspace [`TODO.md`](../../TODO.md) — the `dispatcher` section plus
the **Cross-service / contract** items that constrain dispatcher work
(exit-code plumbing, Sandbox exclusion, command channel).

# process_supervisor

The executor. Pulls assigned processes from dispatcher, spawns PHP workers,
enforces a concurrency cap, and reports results back. Designed to run as many
stateless replicas under Kubernetes.

Binary: `process_supervisor` (`src/bin/process_supervisor.rs`).

## Responsibilities

1. **Fill empty slots.** Up to `MAX_CHILDREN_COUNT` PHP children run in parallel.
   Whenever there is free capacity, supervisor calls
   `GET /obtain_new_process/{supervisor_id}` on dispatcher and launches
   `php worker/worker.php` for each received `AssignedProcess`.
2. **Track child lifecycle.** The main loop periodically polls `try_wait()` on
   each child, and when a child exits, reports the result to dispatcher via
   `PATCH /report_process_finish/{process_id}`, then drops the child from the
   in-memory map.
3. **Drain on shutdown.** When Kubernetes sets `metadata.deletionTimestamp` on
   the pod, a kube-rs controller flips supervisor into **drain mode**:
   populate-empty-slots stops, in-flight children are allowed to finish
   naturally, and only once the process map is empty does the pod remove its
   finalizer and exit. This is what guarantees no lost work on rolling
   deploys / scale-down.

## Lifecycle states

```
          ┌───────────┐
          │  running  │  populating slots, reporting finishes
          └─────┬─────┘
                │ k8s deletionTimestamp set
                ▼
          ┌───────────┐
          │   drain   │  no new work, wait for children to finish
          └─────┬─────┘
                │ processes map is empty
                ▼
          ┌───────────┐
          │ finished  │  mark pod annotation, remove finalizer, exit(0)
          └───────────┘
```

A separate `terminate` annotation flips `is_terminate_mode` (intended for
force-kill), but the actual kill-everything path is **not yet wired up** — see
"Known gaps".

## Concurrency model

- `Supervisor` holds `processes: Arc<RwLock<HashMap<String, Child>>>` — the
  canonical set of running children, keyed by process UUID string.
- `kill_queue: Arc<RwLock<HashMap<String, u64>>>` — ids scheduled for SIGKILL
  after a SIGTERM grace period; drained by a dedicated background task every
  5 s (`process_kill_queue` → `kill`).
- The main populate/reap task runs every 30 s:
  1. `process_states()` — reap finished children, report them, remove from
     the map.
  2. If in k8s **and** drain mode **and** no children — mark pod finished
     and exit.
  3. Otherwise `populate_empty_slots()` until full or drain-mode trips.
- The k8s controller runs in its own task and mutates `is_drain_mode` /
  `is_terminate_mode` via `RwLock`.

## HTTP client to dispatcher

`DispatcherClient` (`src/dispatcher.rs`) — a thin `reqwest` wrapper.

| Method | Env | Default |
|---|---|---|
| obtain | `OBTAIN_PROCESS_URL` | `http://…/obtain_new_process/{supervisor_id}` |
| report | `REPORT_PROCESS_FINISH_URL` | `http://…/report_process_finish/{process_id}` |

The `{supervisor_id}` / `{process_id}` placeholders are replaced at call time.
`supervisor_id` comes from `HOST_NAME` (in k8s: the pod name).

## Kubernetes integration

`src/k8s/k8s_common.rs` / `src/k8s/k8s_supervisor.rs`.

- On startup: read `/var/run/secrets/kubernetes.io/serviceaccount/namespace`
  and `$HOSTNAME` to self-identify. If either is missing, the k8s cycle is
  skipped entirely and the supervisor runs as a plain local process (for
  docker-compose / dev).
- A kube-rs `Controller<Pod>` is scoped to the current pod via
  `metadata.name={pod_name}`. On reconcile it:
  - Adds a finalizer `process-supervisor/finalizer` if missing — this blocks
    k8s from actually deleting the pod until supervisor is ready.
  - Detects `deletionTimestamp` → applies a `drain=true` annotation and sets
    `is_drain_mode`.
  - Detects `terminate=true` annotation → sets `is_terminate_mode`.
- On clean shutdown: patch `finished=true` annotation, patch finalizers to
  `null`, `std::process::exit(0)`.

Manifests live under `ops/k8s/minikube/`:
`deployment`, `service`, `role`/`rolebinding`/`serviceaccount`,
`podmonitor`, `scaledobject` (KEDA).

## Legacy local HTTP API

A small hyper-based server is bound on `HTTP_PORT` (default `8080`):

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/launch/{id}` | Manually spawn a PHP child. |
| `POST` | `/terminate/{id}` | Send SIGTERM; queue SIGKILL after `SIGTERM_TIMEOUT_SECS`. |
| `POST` | `/kill/{id}` | Legacy blocking SIGTERM → sleep → SIGKILL path (`kill_old`). |
| `GET`  | `/state-list` | Snapshot of all children (PIDs, RSS, exit codes). |

**This API predates the dispatcher integration** and is kept only for manual
testing during development. Planned future direction: users trigger commands
through dispatcher, supervisor polls dispatcher for them, and this HTTP
server is removed.

## Environment variables

| Var | Required | Default | Purpose |
|---|---|---|---|
| `HTTP_PORT` | no | `8080` | Legacy debug HTTP server port. |
| `MAX_CHILDREN_COUNT` | no | `10` | Max concurrent PHP workers per supervisor. |
| `SIGTERM_TIMEOUT_SECS` | no | `20` | Grace between SIGTERM and SIGKILL. |
| `OBTAIN_PROCESS_URL` | no | cluster-local default | Dispatcher obtain endpoint template. |
| `REPORT_PROCESS_FINISH_URL` | no | cluster-local default | Dispatcher report endpoint template. |
| `HOST_NAME` | **yes** | — | Used as the supervisor id. In k8s set to `metadata.name` via the downward API (or rely on the default `$HOSTNAME`). |
| `RUST_LOG` | no | `trace` | `tracing-subscriber` env filter. |

## Notable modules

| File | What's inside |
|---|---|
| `src/bin/process_supervisor.rs` | Entrypoint: tracing, env, k8s params probe, kill-queue task, populate/reap task, HTTP server. |
| `src/supervisor.rs` | `Supervisor` struct, spawn/terminate/kill, `process_states`, `populate_empty_slots`, drain/terminate flags. |
| `src/supervisor/results.rs` | Result DTOs for launch/terminate/kill. |
| `src/dispatcher.rs` | `DispatcherClient` (reqwest). Re-exports `AssignedProcess`, `ProcessFinishReport`, status consts from `shared`. |
| `src/k8s/k8s_common.rs` | Pod/namespace/client bootstrap, annotation parsing. |
| `src/k8s/k8s_supervisor.rs` | kube-rs controller, reconcile loop, finalizer handling. |
| `src/server/http/` | Legacy hyper-based local API (`/launch`, `/terminate`, `/kill`, `/state-list`). |
| `worker/worker.php` | Placeholder PHP worker; real worker will be injected via the image. |

## Known gaps / TODO

See the workspace [`TODO.md`](../../TODO.md) — the `supervisor` section plus
the **Cross-service / contract** items (exit-code plumbing, dispatcher
command channel that eventually replaces the legacy HTTP API).

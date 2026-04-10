# Known gaps / TODO

Single source of truth for tracked gaps in the workspace. Grouped by scope —
cross-service items come first because they block or constrain the per-crate
work below.

Convention: each item has a one-line summary followed by a short "why it
matters" note. Check `[x]` when done.

---

## Cross-service / contract

Items that touch **more than one crate** — usually the `shared` wire types plus
both sides that speak the protocol. Changing any of these requires rebuilding
and coordinated deploy of both services.

- [ ] **Carry the child exit code end-to-end.**
  Today `ProcessFinishReport` has only `result: "success" | "error"`; the PHP
  exit code is collapsed in `supervisor::process_states` and never reaches
  dispatcher or the DB. Fix touches: `shared::ProcessFinishReport`,
  `supervisor::supervisor::process_states`, `dispatcher::report_process_finish`,
  `dispatcher_processes` schema (new column).

- [ ] **Mutual exclusion of `Regular` and `Sandbox` per source.**
  `ProcessingMode::Sandbox` exists in `shared` but is never produced, never
  enforced, and running both modes concurrently for the same source can
  corrupt data. Dispatcher must refuse to create a Regular process while a
  Sandbox one is active for the same source (and vice versa). Touches:
  dispatcher scheduling + possibly shared semantics of the enum.

- [ ] **Dispatcher-owned command channel (replacement for supervisor HTTP).**
  Planned direction: human-triggered commands (terminate / kill a specific
  process) are POSTed to a new dispatcher REST endpoint, and supervisor polls
  dispatcher for pending commands instead of exposing its own HTTP. When this
  lands, the entire `crates/supervisor/src/server/http/` and its dual kill
  paths go away. New endpoints + new shared DTOs required.

---

## shared

- [ ] **`DispatchState::new` / `ProcessingMode::new` panic on unknown input.**
  Safe while DB and enums are both owned here, brittle if ever fed untrusted
  input. Revisit if the DB schema starts being written by anything other than
  dispatcher.

- [ ] **`DispatchState` has two serialization conventions.**
  DB text is lowercase (`"created"`, via `Display` + `DispatchState::new`),
  JSON is PascalCase (`"Created"`, via default `serde` derive). Nothing today
  crosses the two planes, but it is a landmine for any future consumer that
  reads both the DB and the HTTP API — for example, a SQL query written from
  what a human sees in logs will silently match nothing. Cleanest fix: add
  `#[serde(rename_all = "lowercase")]` on the enum so JSON matches the DB
  form. That is a breaking change on the wire, but both sides live in the
  workspace and will fail at compile time — which is exactly what the
  `shared` crate is for. Coordinate with any external JSON consumer before
  applying. `ProcessingMode` has the same shape (PascalCase JSON vs numeric
  DB column) but the two planes are fully disjoint there, so it is lower
  priority.

---

## dispatcher

- [ ] **`sources` priority fields are ignored.**
  The external `sources` table exposes `matching_group`, `matching_prio`,
  `matching_order` — dispatcher currently iterates in DB order. Scheduling
  should honor priority so high-priority sources get processes created first
  under load.

- [ ] **No pause between successful schedule cycles.**
  `prepare_schedule` only sleeps 60 s when the previous cycle produced zero
  rows. Non-zero cycles loop immediately. Add a bounded main-loop tick.

- [ ] **`TIMEZONE = "Europe/Berlin"` is hardcoded** in `src/dispatcher.rs`.
  Move to env. The "one finished process per day" check depends on this.

- [ ] **Cancellation token is passed through every method.**
  `src/dispatcher.rs` has a `TODO: move cancellation_token here and use as
  dispatcher property`. Cosmetic, but worth doing before the code grows more
  branches that need cancellation.

- [ ] **Unexpected DB column values trigger `panic!`** (`MySqlRowExt::get_string`,
  enum `::new` methods). Acceptable today because the DB and the code evolve
  together; promote to `Result` if schema ownership ever splits.

- [ ] **Sandbox scheduling — dispatcher side.**
  See the cross-service item above. This is where the enforcement has to live.

---

## supervisor

- [ ] **`worker.php` is a placeholder** — it allocates and sleeps 30 s. Real
  worker will be injected via the image. `Supervisor::launch` hardcodes
  `php worker/worker.php`; make the command configurable via env.

- [ ] **Exit codes are discarded — supervisor side.**
  `process_states` maps any non-zero to `REPORT_STATUS_ERROR` and drops the
  numeric code. Plumb it once `ProcessFinishReport` is extended (see
  cross-service item).

- [ ] **`is_terminate_mode` is observed but no code acts on it.**
  Expected behavior: iterate the process map and force-kill everything when
  the flag flips (with SIGTERM → grace → SIGKILL). Today it is purely a
  status flag set from the pod annotation.

- [ ] **Two kill paths coexist.** `kill_old` (blocking, driven by legacy
  `POST /kill/{id}` HTTP) vs `kill` + `kill_queue` (non-blocking, driven by
  the background task in `process_supervisor.rs`). Unify as soon as the
  legacy HTTP API is removed (see cross-service item).

- [ ] **`populate_empty_slots` is serial.** 1 s sleep + 1 HTTP round-trip per
  empty slot, so a cold supervisor with `MAX_CHILDREN_COUNT = 10` takes ~10
  round trips to fill. Batch the obtain call or parallelize.

- [ ] **Wall-clock drift risk in SIGKILL grace.**
  `terminate_signal_time` is stored as Unix seconds and compared against
  `SystemTime::now()`. Clock jumps can shorten or stretch the grace window.
  Prefer a monotonic `Instant`.

- [ ] **`rss_anon_memory_kb` is Linux-only.** `procfs` is compiled out on
  macOS, so `ChildState.rss_anon_memory_kb` is always `None` there. Fine for
  production (k8s = Linux), note it for local dev.

- [ ] **Stale `TODO: report kill status to dispatcher`** in
  `src/supervisor.rs::process_states` — decide what "killed by supervisor"
  should look like as a terminal state (new `DispatchState`? new `result`
  value? separate endpoint?). Ties into the exit-code plumbing.

- [ ] **Legacy debug HTTP API (`/launch`, `/terminate`, `/kill`, `/state-list`).**
  Predates dispatcher integration. Kept for manual testing. Remove once the
  dispatcher command channel lands (see cross-service item).

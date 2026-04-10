# shared

Wire types exchanged between `dispatcher` and `supervisor`. **No business logic.**

Any change here is a breaking change for both consumers and forces them to be
rebuilt together — this is the whole point of having this crate: the workspace
guarantees both sides can never drift on the protocol.

## Contents

| Item | Purpose |
|---|---|
| `DispatchState` | Lifecycle state of a process row in `dispatcher_processes`: `Created → Pending → Processing → Completed/Failed`. `Error` is a retryable intermediate state reserved for the same supervisor. |
| `ProcessingMode` | `Regular` (1) or `Sandbox` (2). Sandbox is reserved — not produced today. |
| `AssignedProcess` | Payload returned by `GET /obtain_new_process/{supervisor_id}`. Supervisor uses it to spawn a worker. |
| `ProcessFinishReport` | Body of `PATCH /report_process_finish/{process_id}`. Carries `process_id` and `result`. |
| `REPORT_STATUS_SUCCESS` / `REPORT_STATUS_ERROR` | The only valid values for `ProcessFinishReport.result`. |

## Serialization notes

- `AssignedProcess.created_at` uses `chrono::serde::ts_milliseconds` (millisecond epoch).
- `AssignedProcess.mode` is renamed from the Rust field `r#mode` to plain `mode` in JSON.
- `DispatchState` uses **two independent conventions** for the same value:
  - **DB ↔ Rust** goes through `DispatchState::new(&str)` and `Display` —
    lowercase strings (`"created"`, `"pending"`, …) are what lives in the
    `dispatcher_processes.state` column.
  - **Rust ↔ JSON** goes through the default `serde` derive — PascalCase
    (`"Created"`, `"Pending"`, …) is what appears on the HTTP wire.

  Nothing in the current code crosses these two planes, but they **do not
  match**. Do not write raw SQL using the PascalCase form that you see in
  logs or HTTP payloads. Tracked in [`TODO.md`](../../TODO.md).
- `ProcessingMode` currently serializes as the enum variant name, **not** as the numeric
  discriminant. The numeric discriminant is only used for the DB `mode` column (see
  `From<ProcessingMode> for u8`).

## Deliberate constraints

- No `sqlx`, `reqwest`, `axum`, or any runtime-specific dependency.
- Only `serde` + `chrono`. Keep it that way; anything heavier belongs in the
  consumer crate.

## Known gaps / TODO

See the workspace [`TODO.md`](../../TODO.md) — `shared` items and the
**Cross-service / contract** section (any wire-type change lives there).

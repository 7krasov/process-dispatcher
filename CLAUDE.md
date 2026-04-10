# Project context for Claude Code

## Start here

Before doing anything substantial, read:

1. `README.md` — workspace overview, HTTP contract, data flow, how to run locally.
2. `TODO.md` — single source of truth for tracked gaps, grouped by scope
   (cross-service / shared / dispatcher / supervisor). Per-crate READMEs link
   back here; don't duplicate items.
3. The relevant per-crate README under `crates/*/README.md` for the crate you
   are working in.

## Hard rules

- **Do not delete legacy code without asking.**
  Notably `crates/supervisor/src/server/` (the hyper-based `/launch`,
  `/terminate`, `/kill`, `/state-list` API) looks unused but is kept on
  purpose for manual testing until the dispatcher command channel replaces
  it. Exception: during an explicit refactor where new code replaces the
  old within the same change, deletion is fine.
- **Do not write to the `mvp` database.**
  Dispatcher only reads the `sources` table from it; the schema is owned by
  an external legacy project. `crates/dispatcher/db/mvp_migrations/` is a
  **dev-time copy** for bringing the DB up locally, not a migration authority.
  Never add a migration there that changes shape.
- **`worker/worker.php` is a placeholder** — the real worker ships via the
  supervisor image. Do not build features around its current content.
- **Do not touch `crates/supervisor/ops/k8s/`** (or any k8s manifests)
  without explicit request.

## Working environment

- Primary dev environment is `docker-compose up --build` from the repo root.
  This brings up `mysql`, `process_dispatcher`, `process_supervisor` as a unit.
  Dispatcher entrypoint runs both schema migrations before starting the binary.
- MySQL from docker-compose is also exposed on host port `13306`, so it is
  valid to stop the dispatcher/supervisor containers and run those binaries
  from the host via `cargo run`, while keeping the DB in the container.
- The user runs **RustRover**, which continuously runs `cargo check` / `clippy`
  on their behalf and triggers builds on `Cargo.toml` changes. **Do not**
  proactively invoke `cargo check`, `cargo clippy`, `cargo build` after edits.
  Only run them when you specifically need to verify something you just
  changed and are about to assert as done.
- DB migrations are managed via `sqlx` from `crates/dispatcher/Makefile`:
  `make migrate` (dispatcher schema) and `make mvp.migrate` (local mvp copy).

## Commits and git

- Claude drafts commit messages and **proposes the content**; user reviews
  and approves before the commit is created.
- User pushes themselves. **Never `git push` automatically.**
- **Never create PRs automatically.**
- Commit message style, matching existing history and standard git form:
  - Short title line, lowercase, imperative, starting with a verb
    ("fix supervisor for local usage", "migrate dispatcher to tracing",
    "add project documentation"). `fix`, `add`, `remove` etc. as plain
    English verbs are fine.
  - **Do not** use Conventional Commits type prefixes (`feat:`, `fix:`,
    `chore:`, `refactor:`, …) — the repo history does not use that style.
  - If a body is warranted: blank line after title, then plain lines —
    one per point, each starting with a verb. **No `-` or `*` bullet
    markers, no numbering.** Lines capture intent, not a per-file
    changelog — focus on *why* and on anything a future reader would
    need to search for.
  - Inside a refactor commit, describe the refactor's intent rather than
    listing mechanical changes. Mention a rename in the body only when it
    is load-bearing for future archaeology: renamed public API,
    wire-contract field, env var, DB column, or when the rename itself is
    the point of the commit. Internal renames ride along silently.
  - Small changes often need only a title; don't invent a body to fill space.
  - English only, regardless of conversation language.

## Language policy

All code, comments, docs, commit messages, and file contents in English.
Conversation with the user is Ukrainian. This mirrors the global policy and
is non-negotiable for this repo.

## When you are about to make changes

- Prefer editing existing files over creating new ones.
- If a change touches the wire contract between dispatcher and supervisor,
  it must go through `crates/shared` so both sides break at compile time on
  drift. This is the whole reason that crate exists.
- If you add or discover a new gap while working, add it to `TODO.md` under
  the right section instead of leaving a `// TODO` in code — the user wants
  gaps tracked in one place.

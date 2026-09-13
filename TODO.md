# Outstanding agent tasks

- [ ] Migrate the 37 test files and 356 test cases under `src/` into the crate's
  `tests/` directory, removing test-only modules/helpers from production sources.
  Exercise intentional public library APIs; consolidate related cases toward five
  per major feature without losing regression coverage, and explain exceptions
  (§11.3–11.4).

- [ ] Add toolkit-backed scrolling or visible pagination to lists that currently
  stop at the panel boundary: crew, hiring, board, shop/lockup, planning candidates,
  outfit, results, and records. Keep navigation state in the dispatcher and verify
  that large rosters, all eligible equipment, and every result remain reachable
  by touch (§5.1, §7.5).

- [ ] Add a visible run fast-forward control matching Space, and update
  `src/ui/hints.rs`, `src/ui/run.rs`, and `game_page.json` so shortcuts name their
  touch equivalents and tutorial advice names the next visible control. Enlarge
  cramped controls such as the 20-pixel “Got it”/Refit and 22-pixel Treat buttons;
  verify a keyboard-free campaign at desktop and narrow browser sizes, replacing
  matching captures directly in `docs/verification/` (§7.5, §12).

- [ ] Move remaining player-facing labels, hints, notifications, and generated
  narrative templates from `src/ui/`, `src/model/`, `src/game/`, and simulation
  modules into typed JSON under `assets/`, loaded through the toolkit. Derive
  numeric tutorial claims from configuration instead of spelling out fixed
  fatigue thresholds (§5.3).

- [ ] Move remaining gameplay/presentation tuning into JSON, including the
  progression and derived-stat coefficients in `src/rules/attributes.rs` and
  playback durations/speed in `src/playback.rs`; retain current behavior and
  deterministic simulation (§5.3).

- [ ] Add project-owned semantic validation to `GameData::load()` before returning
  usable data. Validate starting crew/inventory, target encounter/environment/loot
  references, achievement IDs, probability ranges, costs, and threshold ordering;
  report contextual errors instead of relying only on shipped-data tests or
  silently dropping unknown references. Cover invalid input through public APIs
  while retaining toolkit parsing (§5.3, §6).

- [ ] Remove the data layer's dependency on engine modules: move the tuning/data
  schemas referenced by `GameConfig` and `GameData` (condition, scrutiny, mastery,
  trait rates, and award definitions) into data/model modules, keeping rule
  calculations and award evaluation in their respective services (§2.1).

- [ ] Split functions exceeding 100 lines into cohesive helpers, starting with
  `draw_condition`/`draw_kit` in `src/ui/crew.rs` and `resolve_door` in
  `src/sim/job.rs`. Use named settlement input/result structs to reduce the long
  `settle`/`record_job` signatures and remove or justify their Clippy allowances;
  keep every Rust file below 800 total physical lines (§2.2, §4, §10.2).

- [ ] Handle preference persistence errors: `Preferences::load()` currently
  defaults on every error, and `Game::save_preferences()` discards save failures.
  Keep first-run defaults, log corrupt/unreadable preferences, and notify the
  player when changed settings cannot be saved (§6).

- [ ] Remove unused parameters and suppression statements, including `ctx` in
  results `draw_doors`, `actions` in records `draw`, and redundant `let _ = x` /
  `let _ = was_phase` statements in chrome/runtime; update callers (§1.4).

- [ ] Add missing module-purpose docs to `src/game/{capture,persistence,runtime}.rs`
  and `src/artwork/render.rs`; correct the obsolete non-test-line wording in
  `tests/code_standards.rs` and the playback/content-floor paths in `README.md`.
  Remove the stale all-work-complete claim/history from `gates_todo.md` and retain
  artwork direction in a reference document rather than a task-free TODO file
  (§9, AGENTS shared/project documentation rules).

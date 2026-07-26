# Master Thief

A heist planner. Recruit a crew, buy the gear, read the target, decide who goes
through which door — then watch the plan survive contact with the building, or
not. Jobs resolve as runs of d20 checks against encounter difficulty classes;
the crew is the campaign.

**Status:** M0–M4 landed. The d20 rules engine, the data model, the campaign
session with save/load, the planning screen, the run, and the campaign around
them: hiring, the outfitter, loot, level-up spending, and crew chemistry. A
headless twenty-week campaign is a test, not a claim (`sim::campaign`). Content
volume (M6) and polish (M7) remain.
Read [`gdd.md`](gdd.md) first; it is the source of truth for what to build.

The crate is a library with a thin binary on top (`src/lib.rs` + `src/main.rs`)
so `rules` and `sim` can be exercised headlessly — that is what the ported
tests and the distribution soak in `sim::job` rely on.

### Where things live

| Path | Holds |
| --- | --- |
| `src/model/` | Pure types: attributes, skills, crew, equipment, targets |
| `src/rules/` | The d20 engine and crew chemistry. No macroquad, no I/O, fully tested |
| `src/sim/` | Planning drafts, job resolution, loot, the week, and whole-campaign playthroughs |
| `src/state.rs` | `GameSession`, the save shape, and migration |
| `src/ui/` | View layer only — returns `UiAction`, mutates nothing |
| `src/ui/floorplan.rs` | The building, drawn through `paint` so a test can measure it |
| `src/game/playback.rs` | Replaying a resolved job at a pace a person can read |
| `src/heist_actions.rs` | The one place an intent becomes a change |
| `assets/data/` | Every balance value, dossier, door, and narrative line |

Ported from `game_apps/master_thief/` (React 19 + Zustand). See §0 of the GDD
for the mechanic carry-over table and what was cut.

## Build and run

```powershell
cargo run                     # native debug build
cargo test                    # tests
cargo fmt -- --check          # CI enforces this
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release --target wasm32-unknown-unknown   # WebGL
```

## Publish

```powershell
.\publish.ps1                 # build Windows + WebGL, deploy to the local preview root
.\publish.ps1 -WebGLOnly      # -WindowsOnly, -DeployOnly, -Production (-p), -FTP, -DryRun
```

## Screenshots

```powershell
.\scripts\capture_ui.ps1 -Scenes crew,hiring,board,shop,planning,run,results
```

Drives the headless capture harness through the `MASTER_THIEF_CAPTURE_*` env
vars wired in `src/main.rs`. Output lands in `docs/verification/`.

## Project docs

`AGENTS.md`, `CODE_STANDARDS.md`, `MACROQUAD_TOOLKIT.md`, and
`GAME_DEVELOPMENT_GUIDE.md` are synced copies of the canonical versions in
`rust_management/docs/` — don't hand-edit them. Project-specific guidance goes
here or in `gdd.md`.

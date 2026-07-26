# Master Thief

A heist planner. Recruit a crew, buy the gear, read the target, decide who goes
through which door — then watch the plan survive contact with the building, or
not. Jobs resolve as runs of d20 checks against encounter difficulty classes;
the crew is the campaign.

**Status:** M0–M6 landed; M7 part-way. Achievements, the records screen and its
reputation/notoriety curves are in, and every content axis in GDD §8 has now
reached its full target. The tutorial, audio, and settings are what remain.
Read [`gdd.md`](gdd.md) first; it is the source of truth for what to build.

### Content against GDD §8

| Axis | Authored | Full target |
| --- | ---: | ---: |
| Outcome narrative lines | 400 | 400 |
| Encounter templates | 71 | 70 |
| Equipment templates | 67 | 60 |
| Achievements | 48 | 40 |
| Heist targets | 45 | 45 |
| Recruit archetypes | 40 | 40 |
| Personality traits | 30 | 30 |
| Critical success/failure effects | 56 | 50 |
| Environmental factors | 16 | 15 |

`GameData::inventory()` reports these and `data.rs` holds the full targets as
floors, so content can only grow.

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
.\scripts\capture_ui.ps1 -Scenes crew,hiring,board,shop,planning,run,results,records
```

Drives the headless capture harness through the `MASTER_THIEF_CAPTURE_*` env
vars wired in `src/main.rs`. Output lands in `docs/verification/`.

## Project docs

`AGENTS.md`, `CODE_STANDARDS.md`, `MACROQUAD_TOOLKIT.md`, and
`GAME_DEVELOPMENT_GUIDE.md` are synced copies of the canonical versions in
`rust_management/docs/` — don't hand-edit them. Project-specific guidance goes
here or in `gdd.md`.

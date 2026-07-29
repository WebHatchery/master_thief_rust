# Master Thief

A heist planner. Recruit a crew, buy the gear, read the target, decide who goes
through which door — then watch the plan survive contact with the building, or
not. Jobs resolve as runs of d20 checks against encounter difficulty classes;
the crew is the campaign.

**Status:** M0–M7 landed — the milestone list in `gdd.md` §13 is complete.
Rules engine, planning, run, campaign, delegation reporting, content at the full
GDD §8 targets, achievements, records, tutorial hints, settings and procedurally
generated audio. `publish.ps1` is clean and the capture set covers every screen.

**The week has a clock in it.** The safehouse charges rent scaled to the
outfit's reputation and every hand draws a weekly retainer, so lying low is a
priced decision rather than a free one; unpaid crew lose loyalty, give a week's
notice, and leave. Heat above a threshold rolls each week for the city's
attention — a tail, a raid, or an arrest that puts somebody in a cell until bail
is posted. The three answers are all purchases, and all of them are shown with
their price on the Crew screen's **Outfit** tab.

**Casing tells you which doors can rewrite the job.** A critical can skip the
next encounter or add one that was never in the plan. Scouting a door reveals
whether it is capable of either, alongside its difficulty class — so a
complication is a hazard you can staff against rather than an ambush, and a
blind run is trading that away too. Whatever arrives mid-job is answered by the
best hand already in the building, which makes bringing cover a real hedge.

**The run has a decision in it, made before the dice.** Alongside who stands
where, the plan carries a *standing order*: push on regardless, or pull the crew
out after one or two doors go wrong. Walking forfeits the score — a job the crew
abandon is scored against the whole building and pays a fraction of the cleared
doors — and buys back everything the unopened doors would have cost: injuries,
notoriety, heat, and what the city learns about your methods. Delegated jobs get
no standing order.

**Being tired is a price, not a locked door.** Past the working threshold a hand
is *spent*: still assignable, carrying a named `Running on empty` penalty,
likelier to come back hurt from any door but a flawless one, and losing loyalty
for having been sent. Only injuries take somebody off the roster outright. So a
week of rest is one option against the payroll clock rather than the only one,
and the planning screen warns in words about the half of the cost that never
reaches the dice.

**And the city learns how you work.** Every door the crew open puts its trade on
a file the city keeps — going through one teaches more than being beaten by it —
and once a trade's file is thick enough it adds a named penalty to every check of
that trade, on every mark on the board. It is the one pressure with no purchase
attached: the answers are weeks of not using that trade, or taking marks whose
doors need something else. The board flags the watched doors and quotes how many
quiet weeks would take a point back off them; the planning breakdown carries it
as `Watched: <Trade>`.

### Content against GDD §8

| Axis | Authored | Full target |
| --- | ---: | ---: |
| Outcome narrative lines | 400 | 400 |
| Encounter templates | 71 | 70 |
| Equipment templates | 67 | 60 |
| Achievements | 60 | 40 |
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
| `src/sim/payroll.rs` | The standing weekly bill: upkeep, retainers, missed wages, notice, walkouts |
| `src/sim/law.rs` | What the city does about an outfit it has noticed: tails, raids, custody, bribes, bail |
| `src/state.rs` | `GameSession`, the save shape, and migration |
| `src/ui/` | View layer only — returns `UiAction`, mutates nothing |
| `src/audio.rs` | Every sound effect, rendered from `synth` voices at boot |
| `src/prefs.rs` | Run pacing, sound, and hints — this game's own settings |
| `src/ui/floorplan.rs` | The building, split from the panel and drawn through `paint` so a test can measure it |
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
.\scripts\capture_ui.ps1 -Scenes crew,hiring,board,shop,planning,run,results,records,settings
```

Drives the headless capture harness through the `MASTER_THIEF_CAPTURE_*` env
vars wired in `src/main.rs`. Output lands in `docs/verification/`.

## Project docs

`AGENTS.md`, `CODE_STANDARDS.md`, `MACROQUAD_TOOLKIT.md`, and
`GAME_DEVELOPMENT_GUIDE.md` are synced copies of the canonical versions in
`rust_management/docs/` — don't hand-edit them. Project-specific guidance goes
here or in `gdd.md`.

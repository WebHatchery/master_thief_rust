# Master Thief

A heist planner. Recruit a crew, buy the gear, read the target, decide who goes
through which door — then watch the plan survive contact with the building, or
not. Jobs resolve as runs of d20 checks against encounter difficulty classes;
the crew is the campaign.

**Status:** scaffold. This is the `rust_management/template/` starter with the
project's identity applied — no game code has been written yet. Read
[`gdd.md`](gdd.md) first; it is the source of truth for what to build.

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
.\scripts\capture_ui.ps1 -Scenes crew,briefing,run
```

Drives the headless capture harness through the `MASTER_THIEF_CAPTURE_*` env
vars wired in `src/main.rs`. Output lands in `docs/verification/`.

## Project docs

`AGENTS.md`, `CODE_STANDARDS.md`, `MACROQUAD_TOOLKIT.md`, and
`GAME_DEVELOPMENT_GUIDE.md` are synced copies of the canonical versions in
`rust_management/docs/` — don't hand-edit them. Project-specific guidance goes
here or in `gdd.md`.

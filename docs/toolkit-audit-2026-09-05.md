# Toolkit audit — 5 September 2026

The original review had no named finding. This audit moved the remaining
local generated-sound map, decoding and playback to SoundManager. The nine
authored effects retain their voices, configuration, seeds, nonlooping playback,
mute guard and clamped volume/gain product. Individual decode failures still
leave the other effects available. Existing audio tests cover synthesis,
duration, silence and volume clamping.

Removed the generic local registry loader in `src/data.rs`; its five callers
now use DataRegistry directly with catalogue context and unchanged duplicate
rejection. All other content already uses labeled toolkit embedded loading.

The audit also confirmed toolkit save slots and migration callbacks in
`src/game/persistence.rs`, preference keys in `src/prefs.rs`, SeededRng in
simulation/state and procedural floorplans, Timeline playback, EventBus,
FloatingTextLayer, achievements, notifications and VirtualUi/shared UI helpers.
Save schemas, heist rules, floorplan painting and stable floorplan seed hashing
remain game-owned.

Final validation: 367 checks including existing audio and content tests;
formatting, strict all-target/all-feature Clippy and Rust source-size limits.
Default `publish.ps1` passed Windows/WebGL release builds, packaging with 11
assets, Preview deployment and Project Roost tracking.

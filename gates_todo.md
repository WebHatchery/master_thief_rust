# Master Thief — Gates TODO

Last verified: 23 August 2026

## Outstanding

None.

## Verified complete

- M0–M7 are complete, as recorded in [`README.md`](README.md) and `gdd.md` §13.
- The migration snapshot in `gdd.md` §0 now records crew chemistry and heist loot as
  built systems.
- Crew chemistry is implemented in `src/rules/chemistry.rs` and used by planning,
  delegation, payroll, job settlement, and week progression.
- Heist loot is rolled from each target's `possible_loot` in `src/sim/loot.rs` and
  added to the inventory during job settlement.
- The verification set contains captures for the board, crew, hiring, outfit,
  planning, run, results, records, settings, and shop screens.

No gameplay, content, documentation, or release-validation work is currently outstanding.

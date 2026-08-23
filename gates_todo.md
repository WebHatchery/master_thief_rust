# Master Thief — Gates TODO

Last verified: 23 August 2026

## Outstanding

- [ ] Bring the migration snapshot in [`gdd.md`](gdd.md) §0 up to date. Its rows for
  crew chemistry and heist equipment drops still describe the original unimplemented
  stubs, while the Rust game implements both systems and covers them with tests.

## Verified complete

- M0–M7 are complete, as recorded in [`README.md`](README.md) and `gdd.md` §13.
- Crew chemistry is implemented in `src/rules/chemistry.rs` and used by planning,
  delegation, payroll, job settlement, and week progression.
- Heist loot is rolled from each target's `possible_loot` in `src/sim/loot.rs` and
  added to the inventory during job settlement.
- The verification set contains captures for the board, crew, hiring, outfit,
  planning, run, results, records, settings, and shop screens.

No gameplay, content, or release-validation work is currently outstanding.

# Outstanding agent tasks

- [ ] Migrate the 37 test files and 356 test cases under `src/` into the crate's
  `tests/` directory, removing test-only modules/helpers from production sources.
  Exercise intentional public library APIs; consolidate related cases toward five
  per major feature without losing regression coverage, and explain exceptions
  (§11.3–11.4).

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
  (§9, AGENTS shared/project documentation rules).

## UI_STYLE review — 2026-09-20

Audit/planning only; no UI implementation is included. The existing scrolling
and touch-control tasks are consolidated into U2 and U9 below; all other prior
tasks remain above. There were no checked-off tasks in the existing file.

### Evidence and implementation order

Read the updated local `AGENTS.md`, `UI_STYLE.md`, `CODE_STANDARDS.md`,
`GAME_DEVELOPMENT_GUIDE.md`, `README.md`, and `gdd.md`. No project-local
`PROJECT_AGENTS.md` was found. Inspected screen renderers, the dispatcher,
runtime/playback wiring, floorplan layout/picking, and capture setup.

Visually inspected all ten existing `docs/verification/ui_*.png` captures:
crew, hiring, outfit, board, shop, planning, run, results, records, and settings.
All are 1280×720, with file timestamps of 2026-08-19. These establish visible
findings for those captured states, not a fresh rendering of the current source.
Code findings below were checked against current source. No live game, new
capture, browser resize, or touch interaction was exercised during this audit.
No normal/minimum supported canvas-size declaration was found in README/GDD;
1280×720 is the current logical/default size, not proof of minimum usability.

Implement U1's composition brief first, then the collection access and feedback
work in U2–U3. U4–U8 apply those foundations to individual decisions; U9 completes
input/help coverage. U10 is the final verification gate. Prefer toolkit layout,
scrolling, camera, and pointer helpers; keep selection/disclosure/navigation state
owned by the dispatcher and return UI intents. Preserve named modifiers, scouting
knowledge boundaries, costs, warnings, and recovery. Do not change heist rules
or introduce mid-run abort/re-roll controls to solve presentation problems.

### Verified findings — ordered by player impact and dependencies

- [ ] **U1 — Recompose the shared shell around the current phase and separate utilities.**
  **Scope:** All screens; `src/ui.rs::{content_rect,draw_game_ui}`,
  `src/ui/chrome.rs::{draw_header,draw_tabs,draw_footer}`, `src/heist_actions.rs::apply`,
  `src/game/runtime.rs`, and README/GDD screen-flow sections.
  **Evidence/problem:** Every capture carries a strong bordered logo/stat header,
  five tabs, hint strip, two main panels, and a six-button footer. Advance Week is
  in the same surface as Save, Load, New Campaign, Delete Save, and Settings; Save
  competes with Commit/Plan the job. Content is fixed at y=166, height=472, even
  when hints are hidden. Code always renders these controls during Planning/Run;
  `ShowScreen` changes screens directly and Advance Week has no run-phase guard.
  **Change:** Write the seven-question UI_STYLE §1 brief for each phase. Declare
  normal and minimum supported canvas sizes; use 1280×720 as the normal baseline
  and evaluate 960×540 landscape and 390×844 portrait as candidate small sizes,
  not already-supported claims. Replace the permanent utility footer with a
  quiet, labelled menu with visible return/recovery controls. Put Advance Week
  in a separate between-job decision area beside wages/runway and relevant
  consequences. During Planning/Run, expose phase-appropriate navigation and
  define guarded leave/resume behavior so tabs cannot strand the active replay.
  Reduce the header to phase-relevant state; move full financial/law breakdowns
  to Outfit and keep urgent shortages near the constrained action. Reclaim the
  removed bands for play rather than leaving blank space. Keep current totals
  distinct from result deltas (U3).
  **Acceptance:** At most 2–3 regions demand strong attention; the current
  decision and advancing action are obvious within a second. Utilities never
  share a gameplay action group. Week advancement cannot silently alter a plan
  or run in progress; menu and recovery remain reachable.
  **Verify:** Compare every phase at normal and declared minimum sizes, with hints
  on/off, high values, no save/existing save, and retirement. Tap Board → Plan →
  menu → return → Commit → Results; verify no lost draft/replay or unintended week
  change. Exercise save/load/new/delete using disposable campaign data.

- [ ] **U2 — Make complete collections reachable without shrinking their contents.**
  **Scope:** `src/ui/{crew,hiring,board,shop,planning,outfit,results,records}.rs`,
  `src/ui/chrome.rs::draw_modifier_grid`, selection state and UI actions.
  **Evidence/problem:** List loops stop at panel boundaries. Shop visibly reports
  “59 more in stock” and Records “+6 earlier jobs” without access controls.
  Planning only fits three 96px candidates; the shelf fits very few spare items.
  Results substitutes a count for later doors. Modifier overflow becomes a
  noninteractive “+N more”. Outfit allocates only 62px to custody at the current
  layout, but its first record needs 68px and is skipped. Unhappy crew start even
  lower, and that list is only called when custody is empty (code-verified).
  **Change:** Add toolkit-backed scrolling or visible pagination, clipped rendering
  and matching hit areas to each collection, including Board door lists, all
  candidates, custody/bonus recipients, results, and modifier details. Reserve
  action space outside scroll content. Retain selection across pages and clamp
  offsets when buying, hiring, selling, or changing targets removes rows. Do not
  confuse intentionally unaffordable shop filtering with inaccessible eligible
  stock. Use the screen-specific layouts below instead of preserving undersized
  panels just to add scrollbars.
  **Acceptance:** Every eligible item/member/door/result and every necessary
  recovery action can be reached by touch; overflow counts lead to actual content.
  No offscreen or clipped control activates; scrolling never accidentally buys,
  hires, sells, or assigns.
  **Verify:** At normal/minimum sizes, reach first and last entries with a roster
  over four, more than three candidates, all affordable stock, a shelf over two
  items, several custody/notice records, a long job and over six history entries.
  Exercise drag and visible paging controls, then mutate the last page's data.

- [ ] **U3 — Reveal job feedback at playback time and retain important weekly events.**
  **Scope:** Run/Results and week advancement;
  `src/heist_actions/jobs.rs::{commit_plan,delegate_job,announce,advance_week}`,
  `src/game/runtime.rs::{start_run,advance_run,finish_run}`, `src/ui/chrome.rs`,
  `src/state.rs`, and `src/ui/records.rs`.
  **Evidence/problem (code):** `run_job` settles the whole job, then `announce`
  reports its outcome/payout and awards before `StartRun`. The header reads the
  settled session during playback, as the existing run capture also illustrates.
  The ending can be revealed before its dice. `advance_week` sends ledger,
  warnings, and notes only to expiring notifications; stored job-history rows
  cannot recover the weekly explanation of raids, rivals, notice, or walkouts.
  **Change:** Keep authoritative deterministic settlement intact, but defer
  report-derived announcements and final totals to their presentation phase;
  omit unrelated totals during playback or clearly present a pre-run snapshot.
  Announce each consequence once when revealed, including skip/instant pacing.
  Keep current condition/finances persistent in their proper homes. Save a
  retrievable week report for consequential events and provide a quiet visible
  link from the week feedback/Records, rather than leaving alert prose onscreen.
  **Acceptance:** The first roll does not disclose the final payout/outcome.
  Skipping yields one complete result, not duplicate notifications. After toasts
  expire the player can still learn who was taken, who left, what was paid/seized,
  or which rival took a mark, and find the applicable recovery action.
  **Verify:** Watch manual/delegated success, failure and called-off jobs in all
  pacing modes; compare state before/after replay without rerolling. Advance a
  week with payroll shortage and law/rival events, let messages expire, reopen
  the report, then save/load. Check notification placement at normal/minimum sizes.

- [ ] **U4 — Give planning a readable floorplan and a focused candidate comparison.**
  **Scope:** `src/ui/planning.rs::{draw_route,draw_room_label,draw_candidates,
  draw_candidate,draw_route_footer,draw_crew_cut}`, `src/ui/floorplan.rs`,
  `src/ui/chrome.rs::draw_modifier_grid`; depends on U1–U2.
  **Evidence/problem:** The capture has tiny modifier chips, crowded skill/DC
  glyphs, a permanent color legend touching the building, and a bright standing
  order beside several equally large buttons. Code confines the actual building
  to roughly 668×330 and paints every candidate's modifiers at 10 logical pixels.
  Room labels use fixed offsets without reserved text/glyph lanes. Cut total and
  joined reasons share one baseline. No pan/zoom exists; partitioning accepts
  smaller rooms when the available area cannot accommodate the door count.
  **Change:** Enlarge the building and candidate comparison using reclaimed shell
  space. Show concise candidate identity, odds/total and actionable warnings;
  reveal the complete named arithmetic through an obvious tap inspector before
  assignment/commit. Reserve lanes for room number/name, status and assignment;
  move secondary detail and the legend to selected-room/help disclosure. Give
  Commit the main emphasis, keep its unmet-door reason visible, separate Back
  from plan decisions, and give standing orders a clear selected state rather
  than blanket danger styling. Keep net take/cut beside Commit, with reasons
  available on tap. Fit the full route at a useful default scale; if dense jobs
  still cannot fit, add visible zoom/fit controls and touch pan via toolkit helpers.
  **Acceptance:** All doors can be identified and selected; selection and casing
  are distinct even for an unknown focused door. Every modifier is readable on
  demand, without exposing unknown DCs/odds. Spent/injured warnings and critical
  telegraphs remain available before commitment; primary action stays reachable.
  **Verify:** Normal/minimum sizes with blind, partly cased and fully cased marks,
  longest labels, largest authored route, >3 candidates and dense modifiers.
  Tap each room, inspect/assign a lower-ranked member, change nerve, clear/reassign,
  and commit. Recheck picking after resize and any zoom; capture selected/expanded
  states. Do not treat a 100px geometric room bound as proof of readable content.

- [ ] **U5 — Make Board rows readable and quote consequences before choosing a job.**
  **Scope:** `src/ui/board.rs::{draw_board,draw_detail,draw_doors,draw_summary}`,
  `src/heist_actions/jobs.rs::delegate_job`; depends on U1–U2.
  **Evidence/problem:** In `ui_board.png`, casing text and the right-aligned
  expiry/ripening text overlap in all three rows. Detail content is surrounded by
  repeated bordered rows while actions sit below a large empty gap. Delegate
  immediately runs a plan: its spent-crew warning is posted after the choice,
  and the Board does not quote the delegation cut. Disabled plan/delegate buttons
  do not explain the no-fit-crew condition at the control.
  **Change:** Use measured, separate row lanes for name/payout and casing/window;
  put extended ripening details in the selected dossier without hiding expiry or
  material risk. Recompose the dossier around doors and the job decision, removing
  redundant container headings/borders. Keep Scout cash and hour costs, watched
  trade penalties, critical risks and rival odds beside the choices they affect.
  Show a compact delegation preview with cut/net estimate, assigned spent hands,
  and lack of standing order before execution. Explain disabled actions locally.
  **Acceptance:** Comparing marks needs no deciphering overlapped text. A player
  knows the price and risk of scouting, planning, delegating or waiting before
  activating that action; undiscovered door facts remain undiscovered.
  **Verify:** At normal/minimum sizes test long names, ripened/expiring marks,
  zero cash/hours, fully cased marks, no fit crew and a delegated spent hand.
  Tap Scout → inspect newly known door → Plan, and separately inspect Delegate
  terms → back out/execute. Keep the complete door list reachable via U2.

- [ ] **U6 — Make Crew tabs change the main decision, not just the narrow list.**
  **Scope:** `src/ui/crew.rs::{draw,draw_dossier,draw_condition,draw_kit}`,
  `src/ui/hiring.rs`, `src/ui/outfit.rs`; depends on U1–U2.
  **Evidence/problem:** Hiring and Outfit captures still devote the large right
  panel to Vera's attributes, kit and red dismissal button. Recruiting and law/
  payroll decisions are squeezed into 340px of usable width. On the dossier the
  portrait lies behind condition rows, and dense chemistry text reaches below
  the panel. Empty injury status is red even when nothing is wrong.
  **Change:** Payroll uses roster + selected member; For Hire uses recruit
  comparison + selected recruit with signing fee, retainer and runway beside
  Hire; Outfit uses the main area for payroll/law and actionable custody/notice
  cases, including simultaneous custody and unhappy crew. Remove the unrelated
  dossier from those latter tabs. In the roster
  dossier give portrait/name/condition distinct space; reveal training, full
  attributes, kit details and chemistry through labelled sections as needed.
  Keep training points/hours and urgent condition visible. Move dismissal into
  deliberate member management with severance and loyalty consequences; preserve
  retirement and bail routes. Use neutral styling for healthy/empty status.
  **Acceptance:** Each tab has one evident purpose; a recruit's own information
  is inspectable before hiring. Injuries, bail and threatened departures are
  never displaced by portrait, kit or advanced stats. Recovery costs stay visible.
  **Verify:** Normal/minimum sizes, long names, many recruits, points with zero
  hours, multiple injuries, worn kit, several prisoners and notice cases. Tap
  through recruit inspection/hiring, training, Treat/Refit, bonus/bail, and back;
  verify correct member identity and refreshed costs throughout.

- [ ] **U7 — Turn the Outfitter into a purchase/equip comparison for an explicit recipient.**
  **Scope:** `src/ui/shop.rs::{draw_catalogue,draw_lockup,bonus_line}`, selection
  actions in `src/ui.rs`/`src/heist_actions.rs`; depends on U1–U2 and U6.
  **Evidence/problem:** Shop capture shows eight equal Buy buttons, bonus text
  running under item art/actions, and five permanent slot cards squeezing the
  shelf. The recipient can only be changed on Crew. Code disables Issue for
  class/level requirements but always labels the failure “needs level”, including
  class-only restrictions; sale price is visible but the per-sale heat consequence
  is not quoted beside Sell.
  **Change:** Add a visible recipient selector in the shop, concise catalogue rows
  with slot/skill filters, and a selected-item comparison showing equipped item,
  bonuses, wear, requirements and affordable price. Put Buy/Issue beside that
  comparison; keep shelf access and selection across recipient changes. Collapse
  irrelevant empty-slot cards. Quote sell return and heat cost near Sell, and
  show the specific unmet Issue requirement. Preserve affordable filtering while
  providing a deliberate route to inspect unavailable equipment if offered.
  **Acceptance:** Players can equip two different crew members without leaving
  the screen and understand the improvement, restriction and transaction cost.
  Item names/bonuses are readable; every eligible item is reachable via U2.
  **Verify:** At normal/minimum sizes buy/issue/take back/sell using two recipients,
  an unusable class/level item, worn gear, low funds, long bonus lists and many
  spares. Dragging lists must not trigger transactions.

- [ ] **U8 — Lead Results with consequences and make Records an inspectable archive.**
  **Scope:** `src/ui/results.rs::{draw,draw_ledger,draw_door}`,
  `src/ui/records.rs::{draw_standing,draw_awards,draw_badge_grid,draw_curves}`;
  depends on U1–U3.
  **Evidence/problem:** Results gives the dominant area to every die/modifier
  while net take has the same weight as seven other ledger rows; loot has an
  enduring reveal-style frame. Code allocates delegation misses at y+238 onward
  and injury/loot detail starting y+244, creating overlapping dense-state ranges
  (not visually reproduced here). Records stacks ten stats, chart, unlabeled
  badge samples and stamps; `draw_badge_grid` takes only four awards per batch
  with no inspection action. Its chart legend relies on color/order to identify
  the two curves.
  **Change:** Start Results with outcome, net proceeds and crew consequences;
  put complete door arithmetic, critical text and delegation lessons behind
  visible selectable rows. Use flowing sections for injuries/loot/misses, and
  quiet persistent loot state after any brief reveal. Keep full results reachable
  via U2. Give Records explicit History/Statistics/Achievements views or disclosure
  controls, a complete named award list with requirement/progress inspection, and
  labelled/distinguishable chart series. Remove stamps repeating visible stats
  when they add no decision or information. Retain seed and retirement outcome
  in the archive without squeezing award detail.
  **Acceptance:** The player immediately understands what the job paid and cost,
  then can inspect every roll and lesson. All awards can be understood without
  guessing glyphs; charts remain interpretable without color. Critical details
  remain readable after animations/toasts end.
  **Verify:** Normal/minimum sizes with successful, failed, called-off and delegated
  reports; multiple misses, injuries and loot; critical added/skipped doors;
  empty/long histories, all award batches and retirement. Tap summary → last door
  → full modifiers → return, and award → requirement → dismiss.

- [ ] **U9 — Replace permanent advice with contextual help and complete touch controls.**
  **Scope:** `src/ui/hints.rs::{hint_for,draw}`, `src/prefs.rs`,
  `src/ui/run.rs::draw_run_controls`, `src/game/runtime.rs::advance_run`,
  `src/ui/{crew,hiring,outfit,shop,settings}.rs`, `game_page.json`; follows U1–U8.
  **Evidence/problem:** Hints persist until globally disabled; Got it disables
  all future hints, including that bar's payroll/custody warnings. Run repeats
  Space-only fast-forward advice in two places despite having only Skip ahead
  and Results buttons. Training targets are 22×18 logical pixels; Got it/Refit
  are 20px high, Treat/bail/bonus 22px, several shop/settings controls 24–26px.
  Global scaling reduces these further at smaller canvases (code inference,
  not a tested touch failure). Startup also announces content counts rather
  than the player's next decision (`src/game.rs::new`).
  **Change:** Track first-use/help completion separately from urgent state
  warnings. Teach exact visible controls, dismiss completed instructions and
  provide labelled reopenable Help; remove implementation prose and content-count
  announcements from normal play. Put warnings beside affected decisions even
  with hints off. Add visible hold-to-fast-forward with release/cancel semantics
  matching Space, distinct from Skip ahead. Reflow/enlarge small controls rather
  than scaling them down; adopt and document a minimum displayed touch target
  (target 44×44 CSS-equivalent pixels with separation). Update browser control copy
  to name touch controls and accurate optional shortcuts.
  **Acceptance:** A keyboard-free player can discover, perform, inspect and dismiss
  every required action. Help dismissal never hides essential warnings. Releasing
  fast-forward stops acceleration; it does not skip the job or change the outcome.
  **Verify:** At actual normal/minimum browser canvases, test first-use/help reopen,
  hints off with shortage/custody, training/Treat/Refit/bail, all settings, hold/
  release/drag-out fast-forward and Skip. Measure displayed hit areas after
  scaling and exercise touch-only navigation, inspection, dismissal and recovery.

### Further inspection — not visually or interactively verified in this audit

- [ ] **U10 — Complete the viewport, dense-state and interaction evidence before accepting the redesign.**
  **Scope:** README/GDD, `src/game/capture.rs`, `scripts/capture_ui.ps1`,
  `src/main.rs`, `src/game/runtime.rs`, `src/ui/floorplan.rs`, browser host and
  `docs/verification/`; depends on U1–U9.
  **Gap/effect:** Existing captures cover only one size and selected fixtures,
  and cannot establish touch behavior, current-build parity, dense failure
  readability, or the declared supported viewport range. GDD camera notes assume
  everything fits; the capture script comment still describes a single boot
  scene although the harness supports multiple scenes. No specific smaller-size
  visual failure or surviving template demo screen is claimed here: the repeated
  shell is verified, but its template origin is not.
  **Change:** Refresh the harness documentation and add reproducible supported
  dense/urgent/expanded fixtures where missing. Record U1's approved viewport
  contract and screen briefs. Run the current build at 1280×720 and the declared
  minimums; explicitly evaluate 960×540 landscape and 390×844 portrait, documenting
  any revised support boundary and an understandable fallback rather than claiming
  success from virtual scaling. Inspect embedded browser canvas as well as native
  output, display scaling, resize and any new pan/zoom transforms. Check settings
  during playback (runtime currently advances it beneath the overlay) and
  keyboard shortcuts while a modal is open; decide/document expected behavior and
  correct any loss of readable progress or unintended underlying actions.
  **Acceptance:** UI_STYLE §9 is satisfied with actual scenes/sizes/input paths
  reported, no more than 2–3 strong attention regions, no lost costs or warnings,
  and no unreachable content. Remaining gaps are explicitly recorded rather than
  treating a compile, capture harness, or geometric test as visual proof.
  **Verify:** Complete a touch-only recruit → buy/equip → scout → plan → commit →
  results → advance-week loop plus urgent recovery and utility return paths.
  Include largest route, long labels/values, dense modifiers, custody/notice,
  retirement and expanded inspectors. Store images directly in
  `docs/verification/`, replacing matching states. After each meaningful game
  implementation change run `./publish.ps1` without parameters and report the
  result/blocker; this TODO-only audit did not run publishing or runtime tests.

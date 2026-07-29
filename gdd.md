# Master Thief — Game Design Document

*Draft v0.1 — living document.*

> You do not pick the lock. You pick the person who picks the lock, buy the tools they'll
> use, read the building until you know which door is the real one, and then commit —
> and every plan is only as good as a d20 you don't get to roll again.

Sources: `game_apps/master_thief/` (React 19 + Zustand original),
`rust_management/migration_candidates.md`, `rust_management/standing.md`,
`rust_management/docs/GAME_DEVELOPMENT_GUIDE.md`, `rust_management/docs/CODE_STANDARDS.md`,
`rust_management/docs/MACROQUAD_TOOLKIT.md`.

---

## 0. Migration Snapshot

- **Old game:** `game_apps/master_thief/` — a React 19 + Zustand SPA (~10.5k LOC TS/TSX,
  no backend). Notable for a real, tested rules engine: `utils/heistExecution.ts` (526
  lines) and `utils/characterCalculations.ts` (318 lines) carry **1,433 lines of tests**
  between them — the most thoroughly tested source app in `game_apps/`.

- **Why it was picked:** `migration_candidates.md` — *"Heist crew-dispatch sim —
  automated-mission structure like `carriage_run`'s expedition meta-game, so art stays to
  icons/UI. Distinct from anything shipped."* The crew-of-specialists-versus-a-fixed-
  obstacle-sequence loop has no analogue in `standing.md`.

- **Art-liability audit.** A full scan of the project for
  `.png/.jpg/.jpeg/.svg/.gif/.webp` returns **zero image files**. No `public/` art
  directory, no icon library, no canvas. Presentation is Tailwind plus five emoji-bearing
  files.

  | Old asset (web) | Art cost | Rust replacement |
  | --- | --- | --- |
  | Crew "portraits" (`EnhancedCharacterCard.tsx`, `TeamMemberCard.tsx`) | None — never existed | Class badge + attribute block + condition meters. A crew member is a personnel dossier |
  | Target/building art | None — a card with a name and payout | A **floorplan drawn from data**: encounters as connected nodes. Procedural line art via `ui` primitives, no assets. See §5.4 |
  | Equipment icons | None — text + rarity colour | Rarity-coloured badges via `colors`, slot glyphs as text |
  | Dice UI (`DiceModal.tsx`) | None — a CSS animation | The most valuable presentation in the game. Drawn with `ui` primitives + `fx`; see §9 |
  | UI chrome | None (Tailwind) | `SurfaceStyle`/`TextStyle`/`GridLayout` |

  **Nothing here requires an artist.** The one genuine visual design problem — making a
  floorplan legible without tile art — is a procedural drawing problem, and the toolkit's
  `paint` module exists precisely so that kind of art can be golden-image tested.

- **Mechanic carry-over table.**

  | Old mechanic | Disposition | Notes |
  | --- | --- | --- |
  | d20 encounter resolution vs. a DC, with attribute modifiers, skill, equipment, condition, environment | **Keep verbatim** | The best asset in the codebase. `resolveEncounter` is a real, tested rules engine — port it as-is, tests and all. See §5.2 |
  | Six attributes (STR/DEX/INT/WIS/CHA/CON) → six derived skills | Keep as-is | Skill = attribute pair + training. Clean, tested |
  | Derived stats (health, stamina, initiative, carry, crit chance/multiplier) | Keep, **pruned** | Carry and the crit stats never came across. Stamina did, and turned out to be consumed by nothing but its own assertion — the instruction was "keep only what a rule consumes", and it took until now to actually apply it to the last one. Health and initiative remain |
  | Seven character classes, five rarity tiers | Keep | |
  | Equipment: 5 slots, 5 rarity tiers, attribute + skill bonuses, special effects, level/class requirements | Keep, **one caveat** | Slots, tiers, bonuses and requirements all work. `special_effects` is thirteen sentences describing thirteen bespoke mechanics that do not exist, and is deliberately never displayed: printing a rule the game does not have is the screen lying about the dice. Implement them or drop them — do not print them |
  | **Manual heists** (`HeistTarget` → `Encounter[]`, resolved one at a time) | **Keep — this is the game** | The tense, legible version of the loop |
  | **Automated heists** (`AutomatedHeist`, team power vs. required power, a timer) | **Redesign, subordinate** | Two parallel resolution systems for the same fiction is the design's central flaw: one is a rich d20 sim, the other a single power comparison against a wall-clock timer. Unified in §5.3 — automation becomes *delegation of the same engine*, never a second engine |
  | Real-time `timeRemaining` on active heists | Cut | Wall-clock timers fight determinism (`CODE_STANDARDS.md` §5) and are a mobile-idle-game idiom this game doesn't want. Jobs resolve when the player advances the week |
  | Character progression: level, XP, attribute/skill points, mastery 0–10 | Keep, **wire** | Mastery came across declared, read by the skill totals, and written by nothing — the same dead-field bug as `characterRelationships`. It is now *earned*: a hand advances a rank by clearing doors of their own trade, at a cost in doors that rises with every rank. Since mastery feeds straight back into that specialty skill, it rewards the one planning decision the greedy best-total sort argues against — giving a specialist their own door when somebody else is marginally better today. Investment against immediate odds |
  | Loyalty, fatigue, injuries, personality traits, backstory events | Keep, **wire** | Loyalty and personality are tracked and barely consumed. Fatigue and injuries are real and good |
  | Relationship system (`characterRelationships`) | **Cut or build properly** | Three `// TODO: Implement full relationship system in Phase 3` sites; the field exists and nothing writes it. Either a real crew-chemistry system (§5.5) or delete the field — no scaffolding |
  | Equipment drops from heists | **Build** | `// TODO: Add equipment finding logic` — `possibleLoot` is declared on every heist and never rolled |
  | Reputation vs. notoriety | Keep, sharpen | Two-axis progression: reputation opens targets, notoriety brings heat. The best under-used idea in the original — see §5.6 |
  | Daily challenges (`expiresAt: Date`) | Cut | Real-world-calendar engagement mechanics don't belong in an offline single-player game |
  | Achievements (5 categories) | Keep | Via toolkit `achievements` |
  | Tutorial (`tutorialSteps.ts`, tested) | Keep | |
  | Outcome description tables (crit fail / fail / neutral / success / crit success) | Keep, expand | The narrative layer over the dice. See §8 |

- **Explicitly out of scope:** multiplayer; real-money or energy mechanics; wall-clock
  timers; character portrait art; a real-time stealth action layer (this is a planner,
  and the plan is the game).

---

## 1. High Concept

- **Pitch:** Assemble a crew of specialists, case a target until you understand which of
  its obstacles will actually stop you, assign the right person to each door — then
  commit, and watch twenty seconds of dice decide whether you were right. The crew
  survives the job; the campaign is what the job does to them.
- **Genre:** Heist crew management / tactical roguelite with a d20 core. Distinct from
  `carriage_run` (escort roguelite — you travel a route) and `frontier` (mission-select
  strategy — abstract resolution). Here resolution is *the content*.
- **Perspective & presentation:** UI-only with one **procedurally drawn floorplan view**
  during a job — encounter nodes connected in sequence, the assigned specialist shown at
  each, lighting up as the run resolves. No camera, no sprites, no tilemap.
- **Tone:** Professional and clipped. Heist-movie competence, not crime-drama grit. The
  crew are tradespeople. Failure is embarrassing before it is tragic.
- **Comparables:** *Monaco* (crew of complementary specialists), *XCOM* (a roster you
  invest in and can permanently lose, percentages you learn to distrust), *Blades in the
  Dark* (the plan is abstract; the roll is concrete).
- **Audience:** Tactics and management players who enjoy visible probability — the
  `carriage_run` audience.
- **Scope:** Full game. See §13.
- **Platforms:** WebGL + native Windows.

---

## 2. Design Pillars

1. **The plan is the play.** All player skill is expressed before the dice roll:
   recruiting, equipping, casing, and assigning. Once committed, the player watches. The
   game must therefore make the *pre-commit* screen the richest one in it.
2. **Every roll is legible.** The player sees the DC, the assigned member's total, and
   every modifier by name — before committing and again in the results. No hidden
   difficulty, ever.
3. **The crew is the campaign.** Levels, mastery, injuries, fatigue, and chemistry
   persist. A perfect job that hospitalises your only hacker is a bad job.
4. **Reputation and notoriety pull in opposite directions.** Every success buys access
   and costs anonymity. There is no strategy that maximises both.
5. **Delegation is a discount, not a shortcut.** Auto-resolving a job runs the same
   engine with worse assignments — never a different, kinder rule.

---

## 3. Core Loop

**The week** is the unit of play.

1. **Case** — review available targets. Casing puts one door at a time on a mark's file,
   front to back, spending both cash and one of the week's limited looks. A mark can be
   half known, and a partly-scouted run is a position rather than a punishment.
2. **Crew** — recruit, rest, treat injuries, spend level-up points, manage chemistry. An
   injury heals free but slowly; *treating* it pays a doctor to buy those weeks back, at a
   price that scales with the wait it saves and leaves the hand tired rather than fresh.
   With a payroll running, a ripening board, and rivals on it, weeks are the expensive
   thing — so the doctor is the same money-for-time trade the rest of the week is made of.
3. **Outfit** — buy, sell, assign, and repair equipment. Spare kit can be moved on
   through a fence at a fraction of list, and the fraction falls as heat rises: nobody
   wants to be seen dealing with an outfit the city is watching. Every sale adds a point
   of heat of its own. It is the week's only inflow that is not a job, deliberately priced
   so it never becomes a better one — a lever for the fixer who cannot make payroll and
   will not take work they have already decided against.

   Kit takes a job's worth of wear every
   time it goes through a door, and worn tools read as a named `Worn kit` penalty on every
   check the hand makes. Refitting is a bill that grows with neglect, so the good tool is
   something the outfit keeps paying to keep good — the ordinary decision every working
   outfit has, and the one thing the Outfitter previously had none of.
4. **Plan** — pick a target and assign a specialist to each encounter node. **This is the
   game's central screen**; it shows every modifier for every candidate at every node.
5. **Commit** — the run resolves encounter by encounter, visibly.
6. **Fallout** — payout, XP, loot, injuries, fatigue, notoriety, heat. Read the results.
7. **Advance the week** — the payroll comes out first, then heat decays, the city rolls
   once for whether it takes an interest, and new targets appear. The week summary reports
   all of it: what was paid, what was short, who gave notice, and what the law did.

Job length: ~1 minute to resolve; a week is 3–5 minutes. A campaign runs 4–8 hours.

---

## 4. Player Role & Verbs

The player is the fixer. Verbs: **recruit**, **let go**, **case**, **equip**, **assign**,
**commit**, **rest**, **treat**, **train** (spend progression points), **delegate**
(auto-assign a job), **advance the week**.

*Let go* exists because *recruit* acquired a permanent price. Once a hand draws a weekly
retainer, taking one on is a standing commitment, and for a while there was no way out of
one: a hand the outfit could not use and could not afford could only be shed by stopping
paying the **whole** crew until that one quit. Rewarding a fixer for starving everybody is
not a decision, it is an exploit. Paying somebody off costs their notice in cash and costs
goodwill with everybody still on the books, because a crew who watch somebody go draw the
obvious conclusion. The last hand standing cannot be let go — an outfit cannot dissolve
itself by accident.

Non-verbs: the player never controls a character in the building, never re-rolls, never
aborts mid-run. Committing is committing.

---

## 5. Systems & Mechanics

### 5.1 Attributes, Skills, Derived Stats

Six attributes on the 3–20 scale with the standard `(score - 10) / 2` modifier. Six
skills, each derived from an attribute pair plus training:

| Skill | Attributes |
| --- | --- |
| Stealth | DEX + WIS |
| Athletics | STR + CON |
| Combat | STR + DEX |
| Lockpicking | DEX + INT |
| Hacking | INT + WIS |
| Social | CHA + WIS |

Ports directly from `characterCalculations.ts`, including its tests.

### 5.2 Encounter Resolution — the d20 core

Ported verbatim from `resolveEncounter`. For an encounter with difficulty class `DC`:

```
total = d20
      + primary skill
      + attribute modifier(s)     (explicit primary_attribute, or the §5.1 pair)
      + equipment bonuses         (skill bonuses + per-encounter slot bonuses)
      + condition modifiers       (fatigue > 50 penalty, injuries, loyalty)
      + crew chemistry            (§5.5)
      + environmental modifiers   (day/night, alarms, weather)
```

Outcome bands: natural 1 → critical failure; `total < DC` → failure; `total >= DC` →
success; natural 20 → critical success. Criticals fire the encounter's
`critical_failure_effect` / `critical_success_reward`, which can restructure the rest of
the run — a critical success on a lock can skip the next encounter entirely; a critical
failure can add one.

**And a scouted door says what a critical there *would* do.** Which doors can rewrite the
run is part of what casing buys, alongside the difficulty class — see §12, question 3. A
complication is answered by the best hand already on the job, never by somebody who was not
in the building.

**A critical says what it did.** The authored effect text is carried on the door outcome
and read out on the run and again in the results, followed by what the critical did to the
plan — the next door opening with this one, or something new waiting. Both halves used to
happen silently: fifty-six effect strings were authored and counted toward the §8 content
target while being read by nothing, and a run that gained or lost a door never said so.
Nothing but a critical carries either line, which is what makes them worth reading.

**The run has one decision in it, and it is made before the dice.** Everything else on the
planning screen is about who stands where; this is about how much has to go wrong before
the crew are told to leave. The fixer sets a **standing order** — push on regardless, walk
after one door goes wrong, walk after two — and the run honours it between doors, never
during one. Deciding in advance rather than mid-run is the point rather than a limitation:
the plan is the game (§1), and a fixer who could call it off *after* seeing the roll would
be playing a different one. It also keeps a committed job a single resolved report that the
run screen replays, rather than a second roll.

Walking is never a win. A job the crew abandon is scored against the *building* — two of
three cleared and out is two thirds of a job, not a clean sweep of what was attempted — and
pays a fraction of what those cleared doors were worth, so leaving late beats leaving early
and both lose badly to finishing. What the forfeited score buys is everything the unopened
doors would have cost: no more injuries, no botched-job notoriety at all and a discount on
the mark's own, less heat, and less taught to the city about the outfit's methods (§5.4).
So a tight order is the cautious, poorer line and no order is the greedy one, which is the
shape a nerve should have. The two shares live in `game_config.json` under `walk_away`.

Delegation gets no standing order. Knowing when to leave is one more thing a fixer who
turns up brings, and one the crew are not given (§5.3) — and *giving* one takes the plan
out of the crew's hands even if they picked every door, so "let them pick, then tell them
when to walk" cannot collect the delegation discount on the cut as well. The label follows
the work, not the button.

**Environmental modifiers must be deterministic.** The original picks day or night with
`Math.random() > 0.5` *inside* the resolution loop (`heistExecution.ts:459`). In the port,
time-of-day is a property of the job chosen at planning time, visible before commit.

### 5.3 Delegation (replacing "automated heists")

The original's second resolution path — team power vs. required power against a wall-clock
timer — is deleted. Delegation instead means: **the game auto-assigns specialists to
nodes using the same greedy best-fit the original used for automation, then runs the
identical §5.2 engine.** The player trades a better assignment for the time they didn't
spend.

**Delegation is what the crew planned, not which button was pressed.** Filling the draft
with "Let them pick" and committing it unchanged produced exactly the assignment the
Delegate button produces, and recorded it as a hand-made plan — so the label was a
formality the player could step around, and with it the differential report. A draft is
the crew's until the fixer argues with it; changing a single door makes it theirs.

**And the crew charge for the thinking.** A plan they drew up themselves carries a named
premium on their cut. Without it, delegation cost nothing at all — the same assignment for
the same money, one click sooner — which made pillar 5 unenforceable. Now delegating is a
priced convenience: worse assignments *and* a bigger share, bought by a fixer who would
rather not plan this one. The results screen states plainly where the auto-assignment differed from the
player's best available option, which teaches the planning screen.

### 5.4 Targets and Floorplans

A target is a name, a difficulty band, a payout, and an ordered list of encounters.
Rendered as a **procedural floorplan**: nodes laid out along a path, connected by
corridors, each labelled with its primary skill, DC (if cased), and assigned specialist.
Drawn entirely with `ui`/`paint` primitives — no tiles, no assets. During a run, nodes
resolve in order and light up green/amber/red with the roll shown.

This is the game's one piece of real visual design, and being data-driven it stays
correct as targets are authored.

**The board is not a stock list.** A mark nobody takes *ripens*: each week it sits, the
score grows by a percentage and every one of its doors gets harder by a fixed step, up to
a cap the closing window arrives before. Both halves are shown on the board — the payout
reads at today's value with the bonus beside it, the difficulty carries its door penalty —
and the penalty appears by name in the planning breakdown as `Mark has ripened`. So
leaving a job for later is a bet the player places knowingly, played against the payroll
clock: the take is bigger, the doors are worse, and the weeks spent waiting cost wages.

**And the crew are not the only people who can see it.** Rivals are other outfits working
the same city — a name and a weekly roll, not a faction and not a content axis. A mark
nobody has touched interests nobody; the longer one sits ripening, the likelier somebody
else takes it, and at most one goes in a week. The board shows the odds per mark, so the
risk is accepted rather than sprung, and the week summary names who got there first and
how many doors of file work went with it. This is the part of waiting that cannot be
calculated: ripening and the closing window are both clocks the player can read exactly,
which made the bet arithmetic. Rivals are the reason to take a job *now* that has nothing
to do with the payroll.

**And the city learns how the outfit works.** Ripening and rivals are both things that
happen *to* a mark; neither of them is the board answering what the crew actually did.
That third thing is **scrutiny**: a file the city keeps per *trade*. Every door the crew
work teaches its trade to every building in town — more for a door they got through than
for one that beat them — and above a threshold the file adds a named penalty to every
check of that trade, anywhere on the board. So a crew who go through the wires often
enough find every set of wires in the city harder, on marks they have not touched.

It is deliberately the one pressure that cannot be bought off. Heat has three answers and
two of them are purchases; a reputation for a method has exactly one, which is weeks of not
using it — or picking marks whose doors need something else, which is the decision the
whole thing exists to create. Both halves are quoted before anything is committed: the
board names the watched doors and says how many quiet weeks would take a point back off
them, and the planning breakdown carries the penalty by name as `Watched: <Trade>`. The
week summary says when a trade has come off the list. The numbers live in
`game_config.json` under `scrutiny`.

### 5.5 Crew Chemistry (replacing the stubbed relationship system)

Either build this properly or delete the field. Proposed: each pair of crew members holds
a chemistry value that moves on shared outcomes — succeeding together raises it, watching
a partner critically fail lowers it, and personality traits set the rate. Chemistry
contributes a small modifier when both members are on the same job, and an extreme
negative pair refuses to work together. This makes the roster a *composition* problem
rather than a sum of individual power ratings, and it gives personality traits — currently
decorative — a mechanical job.

**A contented crew bring in their own work.** Loyalty did three things and every one of
them was a threat — it moved the die, it decided who gave notice, and it decided who held
out for a bigger cut. Keeping a hand happy only ever bought the absence of something bad,
which is a weak pull against a thin week. A hand above a contentment threshold now hears
things: once a week the outfit may be handed a mark that is *not* on the board, with a
couple of its doors already on the file because the person who brought it knows the place,
and a longer window because nobody else is looking at it yet. Every contented hand is
another set of ears, up to a ceiling, so a happy roster generates opportunity instead of
merely surviving. It also makes the board partly a function of who the outfit employs
rather than entirely a function of the seed.

Early on there is nothing to hear: at week one the outfit's name opens three marks and the
board holds five, so the board *is* the city. Tip-offs start mattering exactly when
reputation opens more work than the board can show — which is also about when loyalty has
had time to build.

**A partnership is an asset with a running cost.** Past a threshold a pair stops being two
hands who get on and becomes a *unit*: they read each other at every door, and they know
what a unit is worth when the cut is discussed, so putting them on the same job adds a
named premium. And chemistry is not a permanent acquisition — every week a pair does not
stand in the same building, warmth and grudges alike fade toward indifference. Keeping a
good pair sharp means keeping them together, which is exactly the thing that costs money;
letting them lapse is free and undoes them. A grudge, by the same rule, can be waited out
instead of solved. That is the roster decision chemistry never used to force.

**The crew's cut is negotiated, not fixed.** The retainer buys their week (§5.6); the cut
is what they want for *this* job, and it is read off the roster the plan puts on it. A
base share, plus a step for every hand beyond the first, plus a premium for standing — a
legendary safecracker does not work a job for a beginner's share — plus a premium for
anybody sullen enough to hold out, less a discount for anybody steady enough not to
haggle, clamped at both ends. Every term is named on the planning screen as the draft is
built and again in the results ledger, so the obvious roster — everybody good, on every
door — carries a visible price against the marginal odds it buys, and loyalty gains a
third consequence after the die and the walkout.

### 5.6 Reputation, Notoriety, and Heat

- **Reputation** rises with clean, high-value jobs; gates access to better targets, better
  recruits, and better equipment. It is also what the safehouse costs: upkeep scales with
  reputation, so the outfit's own name is the largest line on its weekly bill.
- **Notoriety** rises with every job and spikes on failures, alarms, and violence. It is
  **monotonic** — see open question 5, settled. It prices bribes and bail, and it closes
  off *people*: a widely known outfit pays danger money to sign anybody, and fewer of them
  bother turning up at all. That is what makes pillar 4 an actual opposition rather than a
  slogan — reputation opens marks, which is a strong pull, so notoriety has to cost the
  other half of the same thing. Before this it only made two rare purchases dearer, which
  meant maximising reputation was very nearly free.
- **Heat** is notoriety's short-term component. It decays weekly, can be bought down, and
  above a threshold it stops being only a difficulty modifier: each week rolls once for
  the city's attention, and a hit is a **tail** (a named penalty on every check for a
  couple of weeks), a **raid** (a share of the outfit's cash seized), or — above a second,
  higher threshold — an **arrest**, which takes the hand the city has seen most of off the
  roster and holds them until somebody posts bail.
- **Scrutiny** is the third axis and the only one that is not a single number: heat is *how
  much* attention the outfit has drawn, and scrutiny is *what the attention is about*. It
  is kept per trade, it is spent down only by not working that trade, and it is described
  in full under 5.4 because what it actually changes is which mark is worth taking.

**The week has a bill.** The safehouse charges rent and every hand on the payroll draws a
retainer scaled to their level and standing, whether or not they worked. A week the outfit
cannot cover is a week the crew go unpaid, and unpaid hands lose loyalty faster each time
it happens; below a threshold they give a week's notice and then leave. The fixer's answers
are all purchases: take a job, pay a bonus to talk somebody round, grease palms to shed
heat, post bail to get somebody back — or let the roster shrink.

Lying low — advancing a week with no job — is a legitimate move and a *priced* one. It
sheds fatigue, closes injuries, and cools the city, and it costs a full week's payroll. The
header carries the two numbers that decide it: what the week costs, and how many quiet
weeks the outfit can still afford.

**Loyalty is graded, not banded.** It contributes a modifier stepping from +2 down to −3
across the 0–100 range rather than two flat bands, so goodwill the outfit burns is visible
on the die immediately. The curve's numbers live in `game_config.json` under `condition`,
alongside fatigue's.

**And exhaustion is a price, not a locked door.** Fatigue used to take a hand off the
roster outright — past a threshold they simply could not be assigned — which quietly made
"rest until everybody is fresh" a move the week could never argue with, however expensive
the wages got. It is now a decision the fixer is allowed to make badly. Past the working
threshold a hand is **spent**: still selectable, carrying a named `Running on empty`
penalty on top of the graded fatigue curve, likelier to come back hurt from any door but a
flawless one, and losing loyalty for every door they were sent through in that state. Only
*injuries* are still a wall, and deliberately so — fatigue is the fixer's call, and a third
broken bone is not. The payroll clock is what gives the choice its teeth: a week of rest
costs a week of wages, and an outfit that cannot afford one now has somewhere to go instead
of nowhere.

Every part of that price is quoted first. The candidate list warns in words — the die can
say what a spent hand is worth on the check, but nothing on a breakdown can say they are
likelier to get hurt — and a delegated job, which skips the planning screen entirely, names
whoever went out on empty. The numbers live under `condition` with the rest of the curve,
including the working threshold itself, which spent a long time sitting at the top of
`game_config.json` being read by nothing while the real bar was a hardcoded 80.

### 5.7 Randomness & Determinism

A single seeded `macroquad_toolkit::rng` owned by the run; the seed is saved and displayed
so runs are reproducible and shareable. All d20 rolls, loot rolls, recruit generation, and
target generation draw from it in a fixed order. No `Math.random()` equivalent anywhere in
the sim — this is the specific bug the original has at `heistExecution.ts:459`.

---

## 6. Data Model (`assets/*.json`)

| File | Defines | Loaded via |
| --- | --- | --- |
| `assets/data/game_config.json` | Starting budget, XP curve, heat decay, DC bands, plus `payroll` (upkeep, retainers, notice thresholds, bonus cost), `law` (attention odds, raid/arrest/surveillance, bribe and bail pricing), and `condition` (the fatigue and loyalty curves) | `load_embedded_json_labeled` |
| `assets/characters.json` | Recruit archetypes: classes (7), rarities (5), attribute ranges, special abilities, personality traits, backgrounds | `DataRegistry` |
| `assets/equipment.json` | Templates (19+), slots (5), rarities (5), bonuses, special effects, requirements | `DataRegistry` |
| `assets/targets.json` | Heist targets: difficulty, payout, encounter sequence, environmental factors | `DataRegistry` |
| `assets/encounters.json` | Reusable encounter templates: primary/secondary skill, DC, complexity, consequences, crit effects, equipment interactions | `DataRegistry` |
| `assets/outcomes.json` | Narrative tables for all five outcome bands, keyed by skill and complexity | `DataRegistry` |
| `assets/achievements.json` | Achievement definitions | `achievements` |
| `assets/strings.json` | UI copy, tutorial steps, crew barks | `DataRegistry` |

Embed-only via `include_str!`, matching `template/`.

---

## 7. World & Progression Structure

- **World layout:** No spatial world. A city as a list of targets that unlock by
  reputation, plus a procedurally drawn floorplan per job (§5.4).
- **Session length:** a week is 3–5 minutes; a campaign 4–8 hours.
- **Progression:** two curves — crew (levels 1–20, mastery 0–10, equipment tiers) and
  operation (reputation tiers unlocking target classes, offset by notoriety/heat).
- **Save model:** `save_to_slot_with_version` / `load_from_slot_with_migration`. Saved:
  run seed, week number, crew roster with full condition state, equipment inventory,
  target availability and casing state, reputation/notoriety/heat, chemistry matrix,
  achievements, job history.

---

## 8. Content Inventory

| Content type | In old game | Prototype target | Full target |
| --- | ---: | ---: | ---: |
| Character classes | 7 | 7 | 7 |
| Recruit archetypes | ~10 | 15 | 40 |
| Personality traits | ~8 | 12 | 30 |
| Equipment templates | 19 | 25 | 60 |
| Encounter templates | ~12 | 25 | 70 |
| Heist targets | 19 | 20 | 45 |
| Environmental factors | ~4 | 8 | 15 |
| Critical success/failure effects | ~6 | 20 | 50 |
| **Outcome narrative lines** | ~25 | 150 | 400 |
| Achievements | ~10 | 15 | 40 |

Outcome lines are the deliberate outlier. Five bands × six skills × complexity tiers is
the game's entire texture, and it is the cheapest content in it to author.

---

## 9. UI/UX & Screen Flow

UI is a pure view layer returning `UiAction`; a `heist_actions.rs` dispatcher applies them.

| Screen | Purpose | Toolkit pieces |
| --- | --- | --- |
| Crew | Roster: attributes, skills, condition, chemistry matrix. Three left-hand tabs: **Payroll** (the roster, each hand showing their weekly retainer), **For Hire**, and **The Outfit** — the books: safehouse upkeep, total retainers, quiet weeks affordable, the odds of a visit from the law, and buttons for the three purchases that answer them (grease palms, pay a bonus, post bail) | `GridLayout`, meters, badges, tooltips |
| Targets | Available jobs, payout, difficulty, casing state | Scroll list, badges |
| **Planning** | Floorplan + per-node assignment with full modifier breakdown for every candidate | `paint`/`ui` primitives, `GridLayout`, `TextStyle` |
| Run | Encounters resolving in order, dice and modifiers shown | `fx`, `timing`, `NotificationManager` |
| Results | Payout, XP, loot, injuries, notoriety delta, narrative lines | Modal surface |
| Shop | Equipment purchase and assignment. **Repair** lives on the crew dossier's Kit section, beside the tools it fixes and next to the treat quote — a refit is a fact about a person's tools, not a line item in a catalogue | Scroll list, badges |
| Records | Job history, crew memorial, seed, statistics | `series` for the reputation/notoriety curves |
| Pause / Settings | | `settings` |

**The dice presentation is load-bearing.** The `DiceModal` is the moment the game's
tension resolves; it needs real timing and weight — a roll that lands, a beat, then the
modifiers totalling up against the DC. Built from `ui` primitives, `fx`, and `timing`, and
skippable (hold to fast-forward) for players on their fortieth job.

Flow: crew/shop/targets freely → planning → commit → run → results → advance week.

---

## 10. Toolkit Mapping

| Need | Toolkit module | Using it? | Notes |
| --- | --- | --- | --- |
| Input handling | `input` | Yes | |
| Widgets/layout/text | `ui` | **Yes — the bulk** | `VirtualUi`, `GridLayout`, `SurfaceStyle`, `TextStyle`, meters, badges, tabs, scroll |
| Textures/manifest | `assets` | No | No textures |
| Camera/pan/zoom | `camera` | No | Floorplans fit one screen by design |
| Cross-system messaging | `events` | Yes | `EventBus<UiAction>` |
| Palette | `colors` | Yes | Rarity and outcome-band colours |
| Vector/grid math | `math` | Yes | Floorplan node layout |
| Frame timing | `timing` | **Yes — critical** | Dice and run pacing |
| Particles/juice | `fx` | **Yes** | Crit success/failure punctuation |
| User settings | `settings` | Yes | Incl. dice-animation speed |
| Unlocks/achievements | `achievements` | Yes | |
| Dev overlay | `debug` | Yes | |
| Deterministic randomness | `rng` | **Yes — critical** | §5.7 |
| Sprite animation | `sprite` | No | |
| Procedural images | `raster` / `paint` | **Yes** | Floorplan drawing; `paint` makes it golden-image testable |
| Headless capture | `capture` | Yes (required) | `MASTER_THIEF_CAPTURE_*`, already wired |
| Save/load | `persistence` | Yes | |
| Tile grid / fog / pathing | `FlatGrid`, `FogState` | No | **Strip the template's grid/fog scaffolding** |
| Charts | `series` | Yes | Reputation/notoriety over the campaign on the records screen |

No toolkit gap identified. If floorplan layout wants a general node-graph layout helper,
raise it as a toolkit upgrade — `mytherra` and `dungeon_core` would both use one.

---

## 11. Architecture Skeleton

```
src/
├── main.rs
├── game.rs                  # Game struct, state machine, transition()
├── game/
│   ├── states.rs            # Menu, Week, Planning, Run, Results, Records
│   └── capture_scenes.rs
├── data.rs                  # embedded JSON + registries
├── data/                    # characters, equipment, targets, encounters, outcomes, strings
├── model.rs                 # Crew, Equipment, Target, Encounter, Job
├── model/
├── rules.rs                 # the ported d20 engine — pure, no engine/UI knowledge
├── rules/
│   ├── attributes.rs        # modifiers, derived stats, skills, level-up
│   ├── encounter.rs         # resolve_encounter + the modifier list
│   ├── chemistry.rs
│   └── loot.rs
├── sim.rs                   # week resolution, heat decay, target refresh, recruit pool
├── sim/
├── state.rs                 # GameSession, SaveData, migration
├── state/
├── ui.rs
├── ui/                      # crew, targets, planning, floorplan, run, results, shop, records
└── heist_actions.rs         # UiAction dispatcher
```

`rules/` must have **no dependency on macroquad**. It is a pure library so the ported
tests run headlessly and a soak test can play thousands of jobs to validate the DC curve.

---

## 12. Non-Goals / Open Questions

**Non-goals:** multiplayer; real-money or energy mechanics; wall-clock timers or daily
challenges; a real-time action layer; character portraits; permadeath-free "safe" mode
(injury and loss are the point).

**Open questions:**

1. **Settled: capture, not death.** A crew member lost to the law is taken into custody —
   off the roster, whole, and recoverable by posting bail. Bail scales with their level and
   the outfit's notoriety, so leaving somebody in a cell is a decision with a running cost
   rather than a fixed one. Crew are also lost to *money*: a hand who goes unpaid long
   enough gives notice and walks, and that loss is permanent. Breaking a captured member
   out as a generated target remains unbuilt and is out of scope for now.
2. **Settled: both, and a door at a time.** Casing is bought per door, front to back, so
   a mark can be half known — the way in scouted and the vault still a rumour. Each door
   costs cash (rising with every door already on that mark's file: the front hall is cheap,
   the vault is not) *and* one of a small fixed number of looks the crew has in a week,
   across the whole board. Money alone can never finish a file, so scouting one building
   properly is scouting every other one not at all. That is what makes blind runs
   interesting rather than merely poor: going in half-lit on a mark you understand is a
   position the player chooses, not one poverty forces on them.
3. **Settled: as much as it likes, once the file says which doors can do it.** The worry
   was the right one and the fix is not to soften the effect — a critical that rewrites the
   run is the most interesting thing the dice do, and a skipped door and an added one are
   worth the same amount of drama. What made adding one feel unfair was that it arrived
   unannounced. So the capability is now part of what casing buys: a door on the file says
   *"Botch this one and something else comes running"* or *"Do this one perfectly and the
   next door opens with it"*, and the board flags a scouted door that can rewrite the run at
   all. Unscouted, the player learns neither — which is one more thing a blind run is
   trading away, and one more reason to spend a look here rather than there (question 2).

   The other half of fairness was who deals with it. A complication had no assignment of its
   own and fell to the first fit name on the *whole payroll* — somebody who was not on the
   job, chosen by roster order, which is not a measure of anything. It now goes to the best
   hand of the people already in the building. That turns a complication into a hazard the
   fixer can staff against: a second capable body on a job is cover, cover costs a share of
   the take and a hand who could have been resting, and it is worth buying exactly on the
   marks whose file says a door can go loud.
4. **Settled: it earns it, now that it costs something.** As a modifier alone chemistry was
   decoration — free upside for a roster the player was picking anyway. It earns its place
   once a partnership carries a premium on the cut and decays when it is not used, because
   those two together turn "who works with whom" into a standing decision with a bill
   attached. The planning screen stayed legible: the pair shows up as one named line among
   the cut's reasons, not as a new panel.
5. **Settled: split the axis, and the campaign ends when the fixer says so.** *Notoriety* is the ledger and never moves down — the
   campaign is finite by design, which is the stronger game. *Heat* is the reducible half:
   it decays on its own, and it can be bought down by greasing palms at a price that rises
   with notoriety, so buying quiet gets steadily worse value as the campaign runs. The
   player therefore has three ways to answer heat, all of them costed — lie low and pay a
   week's wages for a small decay, pay a bribe for an immediate larger cut, or keep working
   and accept the odds of a tail, a raid, or an arrest. None of them touches notoriety.

   Because notoriety only climbs, the campaign has to be able to *stop*, and for a long
   while it could not — the week loop offered another week for as long as anybody kept
   clicking, which is not what "finite by design" means. **Retiring is a verb.** The fixer
   chooses when to get out, and walks away with the cash plus whatever the lockup fetches
   through the same fence everything else goes through — so an outfit that runs for the
   door while the city is watching liquidates at a watched outfit's rate. Cooling off
   before you leave is worth real money, and every extra week worked is more of both. Being
   wrong about when you had enough is the genre.

---

## 13. Milestones

| Milestone | Contents | Done when |
| --- | --- | --- |
| **M0 — Skeleton** | Data model, JSON loaders, `GameSession`, save round-trip, state machine, capture scenes | Load + save/load round-trip tested |
| **M1 — The rules engine** | `rules/` ported from `heistExecution.ts` + `characterCalculations.ts`, **with the original's 1,433 lines of tests translated** | A headless soak test runs 10,000 encounters and the outcome distribution matches expectation |
| **M2 — One job** | Targets, encounters, planning screen with modifier breakdown, run resolution, results | A job can be planned, committed, and resolved end to end |
| **M3 — The floorplan** | Procedural floorplan drawing, node layout, run visualisation, dice presentation | Golden-image tests cover the floorplan renderer |
| **M4 — The campaign** | Weeks, crew progression, injuries/fatigue/rest, equipment shop and loot drops, reputation/notoriety/heat, chemistry | A 20-week campaign is playable and the crew visibly changes |
| **M5 — Delegation** | Auto-assignment via the same engine, differential reporting | Delegated results are never better than a good manual plan |
| **M6 — Content** | Content to the §8 full targets, especially outcome lines | A full campaign rarely repeats a narrative line |
| **M7 — Polish** | Tutorial, achievements, records + `series` charts, accessibility, audio via `synth`, balance | `publish.ps1` clean; CI green; capture set covers every screen |

---

## 14. Verification

- **Port the tests first.** The original's `characterCalculations.test.ts` (802 lines) and
  `heistExecution.test.ts` (631 lines) are the specification. Translating them in M1
  before writing new rules code is the single highest-value step in this port.
- **Determinism test** — same seed + same plan → identical job history, *and* the same
  seed replayed against separately loaded content. The second half is the one with teeth:
  `DataRegistry` is backed by a `HashMap`, so every load iterates in a different order, and
  any draw taken from an unsorted registry would make a seed stop reproducing its campaign
  between runs while every single-load determinism test stayed green. The guard was
  validated by deliberately unsorting one draw and confirming it fails — and confirming the
  older tests do not. Every site that sorts a registry before drawing from it says why.
- **Distribution soak** — a headless run of thousands of jobs asserting success rates by
  difficulty band stay inside designed windows; guards against DC drift as content lands.
- **No-dead-content test** — every encounter template reachable from some target, every
  equipment item purchasable or droppable, every outcome band having lines for every
  skill.
- **Golden-image test** on the floorplan renderer via `paint`.
- **Screenshot capture** per screen via `scripts/capture_ui.ps1`.

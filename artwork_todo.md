# Master Thief — AI Artwork TODO

Status: agent-executable artwork, UI, and validation checklist
Visual direction: **noir payday** — rain-glossed streets, smoked glass,
brass hardware, sodium lamps, paper case files, and the quiet confidence of a
crew that knows exactly what it is doing.

This checklist contains work an AI agent can complete in this repository:
generate raster artwork, implement procedural rendering, update manifests and
tests, capture verification images, and run the project publisher. Every
checkbox is intended to be one bounded task or one small batch of related
assets. Use the current JSON registries as the source of truth for any future
additions; the explicit batch ranges below reflect the current data snapshot.

This is a heist-planning game, not a real-time action game. Art must make the
plan, the people, the building, and the dice legible before it makes them
dramatic. No asset may hide a DC, modifier, assignment, injury, payout, or
required tap target.

The checklist does not track external commissions, human-only art direction or
licensing approvals, or layered editable masters. Runtime PNGs, procedural
renderers, metadata, and repeatable validation are the deliverables here.

## Art direction

### Mood

- Late-night city, professional crew, expensive targets, controlled danger.
- Noir references: wet asphalt, venetian-blind shadows, cigarette-card
  silhouettes, typewritten dossiers, hotel brass, green-shaded desk lamps,
  and blue police light through frosted glass.
- Payday energy: a clean payday board, bold take values, tool rolls, masks,
  gloves, radios, drills, and the visual rhythm of preparing a score.
- The crew are tradespeople first. Avoid torture, gore, skull-heavy outlaw
  cliches, militarized assault imagery, or comic-book villain exaggeration.

### Palette

Use this shared palette; procedural UI colors should harmonize with it rather
than compete with it.

| Role | Color | Hex | Use |
| --- | --- | --- | --- |
| Ink | near-black navy | `#0B1018` | backgrounds, silhouettes, text shadow |
| Safehouse | smoked blue | `#162331` | panels, cards, floorplan voids |
| Paper | warm ivory | `#E7D8B7` | dossier paper, labels, highlighted copy |
| Brass | aged gold | `#C18A3A` | borders, selected state, hardware, payout |
| Neon | cyan-blue | `#3EB7C7` | information, active scan, focus, link lines |
| Warning | amber | `#E29B3F` | heat, caution, partial success |
| Danger | police red | `#C9544D` | alarm, injury, failure, wanted state |
| Success | mint | `#73C2A1` | cleared door, paid, healthy, success |
| Felt | deep burgundy | `#4C1F2A` | rare accents, results, high-value loot |

Do not use color as the sole carrier of state. Pair every color with a glyph,
label, pattern, border, shape, or motion change.

### Rendering language

- Use crisp 2D illustration with strong silhouettes and selective painterly
  grain. Details must survive a 25% reduction.
- Use hard directional light from a desk lamp, street lamp, or police beacon.
- Keep halftone, paper grain, blueprint lines, and rain streaks restrained;
  never put noisy texture behind dense stats or dice results.
- Use procedural geometry for floorplans, node links, dice, meters, buttons,
  tabs, and ordinary UI chrome. Use raster art for identity, atmosphere, and
  authored content.
- Design for the 1280 x 720 logical layout and narrow WebGL windows.

## 1. Inventory and technical scaffolding

- [ ] Read the IDs and relevant fields from `characters.json`,
  `equipment.json`, `targets.json`, `encounters.json`, `environment.json`,
  and `achievements.json`; refresh the explicit batch ranges below if the
  registries change before beginning a batch.
- [ ] Create or update the `assets/images/brand`, `portraits`, `targets`,
  `environments`, `items`, `icons`, `achievements`, `effects`, and `ui`
  folders needed by the current runtime.
- [ ] Apply lowercase `snake_case` filenames with semantic identity first and
  state last; keep prices, DCs, names, positions, and revision numbers out of
  runtime filenames.
- [ ] Add one shared generation prompt block and the palette to
  `assets/artwork_manifest.json`.
- [ ] Add manifest entries for every authored raster family and every
  procedural renderer before adding new UI references.

## 2. Crew portraits and identity

### Registry and class assets

- [ ] Extract the seven current class values from `characters.json` and create
  exactly one readable badge key for each class.
- [ ] Generate the seven class badges and wire each badge to the class lookup.
- [ ] Generate `portrait_unknown.png` and `portrait_locked.png` with matching
  case-file framing and no baked UI copy.
- [ ] Define a portrait recipe that keeps face, age, silhouette, hairstyle,
  clothing cue, and prop distinct across the 40 character IDs.

### Neutral portrait batches

Generate one 4:5 transparent portrait and one dense-list derivative per batch.
Keep each batch small enough to compare identity and lighting against the
previous batch.

Use a 512 x 640 RGBA runtime PNG and a 256 x 320 derivative. Keep the face and
identifying prop inside a 10% safe margin, use a three-quarter or front angle,
and do not bake UI copy into the art.

- [ ] Batch 1: `vera_sloan`, `otis_kemp`, `birdie_lang`, `hollis_pike`,
  `juno_vasquez`, `marcus_dunn`, `sasha_reyes`, `walter_boyd`.
- [ ] Batch 2: `delphine_arceneaux`, `tobias_crane`, `nadia_frost`,
  `gerald_moss`, `imani_okoro`, `rook_maddox`, `priya_raman`, `cyril_mott`.
- [ ] Batch 3: `eda_stroud`, `bram_teague`, `ivy_calder`, `harlan_vise`,
  `nell_faraday`, `desmond_okafor`, `greta_lindqvist`, `aurelio_bassi`.
- [ ] Batch 4: `kit_mahoney`, `solomon_pike`, `yusra_haddad`, `orson_bray`,
  `clemency_dunne`, `matthias_orr`, `rosalind_vane`, `tobias_lench`.
- [ ] Batch 5: `aiko_shimada`, `gaspard_rue`, `wilhelmina_dove`,
  `emeric_shaw`, `petra_almeida`, `leonid_varga`, `beatrix_hollow`,
  `august_vane`.
- [ ] Validate the full neutral set for dimensions, transparency, safe margin,
  filename/ID parity, and small-size readability.

### Portrait state behavior

Use the neutral portrait as the identity source and prefer code-drawn overlays
or controlled tint/posture changes over multiplying the 40-portrait bitmap
set.

- [ ] Implement neutral and speaking states in the portrait renderer.
- [ ] Implement pleased/success and worried/low-loyalty states.
- [ ] Implement injured and exhausted states without using blood as the main
  signal.
- [ ] Implement arrested/absent and unavailable/unknown states.
- [ ] Add class/employment badges that remain readable beside a 16–18 px label.

## 3. Targets, districts, and safehouse atmosphere

Target thumbnails are cards, not explorable maps. Keep encounter doors and node
positions procedural.

Use 640 x 360 RGBA or opaque PNGs with a 16:9 crop, a text-safe zone, and a
focal subject placed according to the screen layout. Use 1920 x 1080 PNG/JPG
environment plates and darken them for UI overlays rather than painting stats
into the plates.

- [ ] Generate target thumbnails for IDs 1–15 from `targets.json`.
- [ ] Generate target thumbnails for IDs 16–30 from `targets.json`.
- [ ] Generate target thumbnails for IDs 31–45 from `targets.json`.
- [ ] Validate all 45 target files for a 16:9 crop, text-safe zone, focal
  subject, dimensions, and target-ID parity.
- [ ] Generate the night/clear environment plate.
- [ ] Generate the night/rain environment plate.
- [ ] Generate the night/fog environment plate.
- [ ] Generate the safehouse desk plate with case folders, brass lamp, radio,
  coffee, lockbox, and a clean negative-space region for cards.
- [ ] Inspect the Board composition: add a map texture only if the screen
  actually uses a map; otherwise document and keep the board procedural.
- [ ] Add darkened 1280 x 720 derivatives for any opaque environment plate used
  beneath UI overlays.

## 4. Equipment and loot icons

The current equipment registry has 67 items, five slot values (`accessory`,
`armor`, `gadget`, `tool`, and `weapon`), and five rarity values
(`basic`, `improved`, `advanced`, `masterwork`, and `legendary`).

Use 64 x 64 RGBA runtime icons with the object centered in a 52 x 52 safe area,
a consistent three-quarter view, no baked names/stats/prices, and a silhouette
that remains recognizable at 24 x 24. Use the shared object drawing with
procedural rarity and state overlays so bitmap variants do not multiply the
art burden.

### Slot and item inventory

- [ ] Generate one slot glyph for each of the five current slot values.
- [ ] Generate item icons for equipment records 1–17.
- [ ] Generate item icons for equipment records 18–34.
- [ ] Generate item icons for equipment records 35–51.
- [ ] Generate item icons for equipment records 52–67.
- [ ] Validate all 67 item files for one-to-one ID parity, centered silhouette,
  64 x 64 runtime dimensions, and recognition at 24 x 24.
- [ ] Add `loot_unknown`, `loot_cash`, `loot_document`, and `loot_jewellery`
  only when the results UI displays those categories.

### Shared item treatments

- [ ] Implement the `basic` and `improved` rarity treatments.
- [ ] Implement the `advanced` and `masterwork` rarity treatments.
- [ ] Implement the `legendary` rarity treatment.
- [ ] Implement equipped and selected overlays.
- [ ] Implement locked/level-gated and newly-found overlays.
- [ ] Implement worn, broken, and sold overlays.
- [ ] Add the item manifest validation so every equipment ID resolves to one
  bitmap or one explicit procedural renderer, never both accidentally.

## 5. Floorplan, doors, and encounters

The floorplan should read as a case-file blueprint. Keep node positions,
connections, and state transitions in code.

- [ ] Add procedural symbols for the six current encounter skills:
  `social`, `lockpicking`, `stealth`, `hacking`, `athletics`, and `combat`.
- [ ] Add complication glyphs for time/light factors: `day`, `dusk`, `night`,
  and `dawn`.
- [ ] Add complication glyphs for weather factors: `clear`, `rain`, `fog`, and
  `storm`.
- [ ] Add complication glyphs for scene factors: `crowded`, `well_lit`,
  `noisy`, and `high_security`.
- [ ] Add complication glyphs for security/context factors: `wired`,
  `old_money`, `understaffed`, and `private_security`.
- [ ] Implement unknown, cased, and locked node states.
- [ ] Implement selected, assigned, ready, and in-progress node states.
- [ ] Implement success, failure, critical-success, and critical-failure node
  states.
- [ ] Implement skipped state and verify it cannot be mistaken for success.
- [ ] Draw door/room silhouettes for lobby, service door, vault, and office.
- [ ] Draw door/room silhouettes for loading dock, roof, alley, elevator, and
  exit.
- [ ] Render route lines as readable red drafting ink or cyan drafting ink
  beneath node highlights.
- [ ] Add an assigned-crew portrait medallion that remains recognizable at node
  size.
- [ ] Add a target-specific floorplan legend with text labels.
- [ ] Add a low-contrast procedural paper/blueprint treatment only if the
  floorplan needs texture after the readable geometry is complete.

## 6. Dice, resolution, and heist effects

- [ ] Draw the ivory/black d20 base and brass edge with readable numbers 1–20
  at 32 px.
- [ ] Add cyan critical-success treatment without obscuring the number.
- [ ] Add red critical-failure crack treatment without obscuring the number.
- [ ] Implement attribute, skill, and equipment modifier chips.
- [ ] Implement condition, environment, and chemistry modifier chips.
- [ ] Implement the paper-snap door transition.
- [ ] Implement the brass-stamp door transition.
- [ ] Implement the cyan scan-sweep door transition.
- [ ] Implement the restrained red alarm pulse.
- [ ] Implement paper-fleck motion only where it does not cover resolution text.
- [ ] Implement cleared and partial/neutral outcome marks.
- [ ] Implement failed, critical-success, and critical-failure outcome marks.
- [ ] Implement the alarm/heat edge treatment as a readable beacon reflection or
  pulse rather than a full-screen flash.
- [ ] Implement the injury/fatigue bandage and clock glyphs with subdued
  portrait changes.
- [ ] Implement the loot reveal as a folder slide, evidence stamp, or
  velvet-lined reveal frame.
- [ ] Add reduced-motion and low-opacity paths for dice, scans, alarms, and
  loot reveals.

## 7. Global UI chrome and iconography

### Brand and navigation

- [ ] Establish the 24 x 24 icon grid, one stroke weight, and one corner
  language before generating the bitmap glyph family.
- [ ] Generate the horizontal Master Thief wordmark at runtime size.
- [ ] Generate the compact monogram and monochrome light/dark variants.
- [ ] Export bitmap glyphs at 48 x 48 RGBA when a procedural glyph is not
  sufficient; keep UI frames and hit-tested surfaces procedural.
- [ ] Wire the wordmark and monogram into title, loading, and catalog-safe
  layouts without baking screen copy into the artwork.
- [ ] Implement the Crew and The Board tab icons.
- [ ] Implement the Outfitter, Last Job, and Records tab icons.

### Generic controls

- [ ] Implement save, load, delete, settings, sound, and music glyphs.
- [ ] Implement fullscreen, help, close, back, warning, info, success, and
  danger glyphs.
- [ ] Implement search, filter, sort, add, remove, lock, unlock, and
  notification glyphs.
- [ ] Implement case file, target, payout, reputation, notoriety, and heat
  glyphs.
- [ ] Implement safehouse, payroll, doctor, fence, bail, retire, door, dice,
  and crew glyphs.
- [ ] Keep unfamiliar icon buttons paired with visible text labels and a
  minimum 44 x 44 logical-pixel touch target.
- [ ] Keep plaque, panel, modal, button, tooltip, footer, and close chrome
  procedural and responsive.

## 8. Records and campaign marks

Achievement identity is data-driven. Do not create a badge or label for a
mechanic that is absent from `achievements.json`.

- [ ] Enumerate the 82 current achievement IDs and group them into four lookup
  batches of no more than 21 IDs.
- [ ] Add the procedural badge renderer for locked, earned, and notable states.
- [ ] Wire achievement lookup batch 1 to the badge renderer.
- [ ] Wire achievement lookup batch 2 to the badge renderer.
- [ ] Wire achievement lookup batch 3 to the badge renderer.
- [ ] Wire achievement lookup batch 4 to the badge renderer.
- [ ] Add campaign record stamps for first job, clean job, botched job,
  legendary loot, retired outfit, heat peak, and long-running crew only when
  each event exists in the simulation data.
- [ ] Validate badge readability at 96 x 96, 48 x 48, and 24 x 24.

## 9. Accessibility, provenance, and asset rules

- [ ] Express every decision-relevant state with at least two of icon, label,
  shape, pattern, position, or motion.
- [ ] Check critical text and glyph contrast against `#0B1018`, `#162331`, and
  `#E7D8B7` surfaces.
- [ ] Verify that portraits and target thumbnails do not cover stats, payout,
  required reputation, or required touch targets.
- [ ] Verify all required actions work through visible touch/click targets;
  keyboard shortcuts remain supplemental.
- [ ] Record the generator, prompt notes, date, and source/runtime relationship
  for each generated asset family in `assets/artwork_manifest.json`.
- [ ] Scan generated images for accidental text, watermarks, broken anatomy,
  broken props, unreadable dice, and inconsistent character identity; regenerate
  or replace failures.
- [ ] Scan generated images for real logos, police insignia, branded weapons,
  copyrighted characters, and celebrity likenesses; regenerate failures.
- [ ] Declare `nearest` filtering for flat icons and `linear` filtering for
  portraits, plates, and soft effects in the manifest.

## 10. Manifest, integration, and validation

### Manifest and tests

- [ ] Keep one manifest entry per shipped runtime texture with a valid path,
  filter, and stable key.
- [ ] Add a test that every character ID resolves to exactly one portrait or an
  explicit procedural renderer.
- [ ] Add a test that every equipment ID resolves to exactly one item asset or
  an explicit procedural renderer.
- [ ] Add a test that every target ID resolves to exactly one target asset.
- [ ] Add a test that every achievement ID resolves to a badge renderer.
- [ ] Add a test that every encounter skill and environment factor used by the
  floorplan has a glyph or documented fallback.
- [ ] Add PNG dimension and alpha-edge checks for generated deliveries.
- [ ] Add a test that rejects duplicate keys and missing manifest paths.

### Screen integration

- [ ] Integrate and verify Crew/Payroll portraits, badges, state marks, and
  dossier treatment.
- [ ] Integrate and verify Crew/For Hire portraits, rarity, cost, and specialty
  markers.
- [ ] Integrate and verify Crew/The Outfit slot glyphs, item states, payroll,
  and safehouse treatment.
- [ ] Integrate and verify The Board target art, payout, heat, notoriety, and
  location treatment.
- [ ] Integrate and verify Outfitter items, slot families, rarity, repair, and
  wear treatment.
- [ ] Integrate and verify Planning target art, floorplan states, routes,
  legends, and candidate portraits.
- [ ] Integrate and verify The Run encounter states, assigned portrait, dice,
  modifiers, alarm, and door effects.
- [ ] Integrate and verify Last Job outcome art, payout, loot, injury/fatigue,
  heat, and reputation changes.
- [ ] Integrate and verify Records badges, milestones, statistics, and tabs.
- [ ] Integrate and verify Settings sound/music/fullscreen/help, pacing, reset,
  delete, and confirmation states.
- [ ] Integrate and verify global title, tabs, save/load/delete, hints, footer,
  tooltips, and modal close controls.

### Repeatable project checks

- [ ] Capture or replace the corresponding image in `docs/verification/` for
  each verified screen; do not create duplicate captures of the same state.
- [ ] Check the final UI at 1280 x 720.
- [ ] Check the final UI at 1024 x 576.
- [ ] Check the final UI in a narrow 800 x 600 window.
- [ ] Run `.\publish.ps1` from the project root after each meaningful art or
  UI batch.
- [ ] Replace the root `catalog_thumbnail.png` with a title-screen or main-menu
  capture when the title composition is ready.

## Definition of done

- [ ] Every authored character, equipment item, target, achievement, encounter
  skill, environment factor, and visible semantic state has a named asset or a
  documented procedural renderer.
- [ ] All primary screens and modal/settings states pass the screen integration
  checklist with no placeholder art.
- [ ] Crew identity is readable in list, detail, planning-node, run, and results
  contexts.
- [ ] Equipment and loot are recognizable at their smallest rendered size.
- [ ] The floorplan and dice sequence remain the visual focus without obscuring
  the numbers that drive decisions.
- [ ] The manifest, asset-key tests, verification captures, and `publish.ps1`
  all pass.
- [ ] The catalog thumbnail uses the title-screen noir safehouse/board
  composition and the shared palette.

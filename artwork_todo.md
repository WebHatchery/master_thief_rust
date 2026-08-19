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
- [x] Record the generator, prompt notes, date, and source/runtime relationship
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
- [x] Integrate and verify Crew/The Outfit slot glyphs, item states, payroll,
  and safehouse treatment.
- [ ] Integrate and verify The Board target art, payout, heat, notoriety, and
  location treatment.
- [x] Integrate and verify Outfitter items, slot families, rarity, repair, and
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

# Master Thief — Artwork Requirements

Status: production brief and asset checklist  
Visual direction: **noir payday** — rain-glossed streets, smoked glass,
brass hardware, sodium lamps, paper case files, and the quiet confidence of a
crew that knows exactly what it is doing.

This is a heist-planning game, not a real-time action game. Art must make the
plan, the people, the building, and the dice legible before it makes them
dramatic. No asset may hide a DC, modifier, assignment, injury, payout, or
required tap target.

## Art direction

### Mood

- Late-night city, professional crew, expensive targets, controlled danger.
- Noir references: wet asphalt, venetian-blind shadows, cigarette-card
  silhouettes, typewritten dossiers, hotel brass, green-shaded desk lamps,
  burgundy felt, and blue police light seen through frosted glass.
- Payday energy: a clean payday board, bold take values, tool rolls, masks,
  gloves, radios, drills, and the visual rhythm of preparing a score.
- The crew are tradespeople first. Avoid torture, gore, skull-heavy outlaw
  cliches, militarized assault imagery, or comic-book villain exaggeration.

### Palette

Use this as the shared art palette; procedural UI colors should harmonize with
it rather than compete with it.

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
label, pattern, border, or motion change.

### Rendering language

- 2D graphic illustration with crisp silhouettes and selective painterly grain.
- Strong outer silhouettes; details should survive a 25% reduction.
- Hard directional light from a desk lamp, street lamp, or police beacon.
- Restrained halftone, paper grain, blueprint lines, and rain streaks; never
  put noisy texture behind dense stats or dice results.
- Use procedural geometry for the building floorplan, node links, dice, meters,
  buttons, tabs, and ordinary UI chrome. Bitmap art is reserved for identity,
  atmosphere, and authored content.
- Artwork must read at the game's 1280 x 720 logical layout and remain usable
  on narrow WebGL windows.

## Current screens and required art coverage

| Screen | Art that must exist | Delivery status |
| --- | --- | --- |
| Crew / Payroll | crew portraits, class badges, injury/fatigue/loyalty marks, dossier paper | TODO |
| Crew / For Hire | recruit portraits, rarity treatment, hire-cost marker, specialty badges | TODO |
| Crew / The Outfit | equipment slot glyphs, worn/broken kit marks, payroll and safehouse motifs | TODO |
| The Board | target category marks, payout stamp, notoriety/heat marks, location thumbnails | TODO |
| Outfitter | item icons, five slot families, five rarity treatments, repair/wear overlay | TODO |
| Planning | target header art, floorplan node states, route/connection treatment, candidate portraits | TODO |
| The Run | encounter node resolution states, assigned-hand portrait, dice faces, modifier chips, alarm/door VFX | TODO |
| Last Job | outcome illustration, payout/loot stamps, injury/fatigue result marks, heat/reputation arrows | TODO |
| Records | achievement badges, campaign milestones, statistics marks, case-file tabs | TODO |
| Settings | sound/music/fullscreen/help icons, pacing controls, reset/delete confirmation marks | TODO |
| Global chrome | title mark, tab icons, save/load/delete, hint strip, footer, tooltips, modal close | TODO |

## 1. Character portraits and crew identity

The data currently defines 40 recruit archetypes and seven classes. Portraits
must support a rotating roster without making every generated recruit look like
the same person.

### Required portrait set

- [ ] Seven class key portraits, one for each class in `assets/data/characters.json`:
  infiltrator/safecracker, muscle/doorman, face/talker, hacker, lookout,
  technician, and the remaining authored class in the data.
- [ ] A reusable portrait recipe for all 40 recruit archetypes. Each recruit
  needs a distinct face, age, silhouette, hairstyle, clothing cue, and prop;
  never distinguish recruits by hair color alone.
- [ ] Portrait states for neutral, speaking, pleased/success, worried/low
  loyalty, injured, exhausted, arrested/absent, and unavailable/unknown.
- [ ] A crew silhouette fallback for an unillustrated recruit and a locked
  silhouette for unrevealed content.
- [ ] Tiny class/employment badges that remain readable beside a 16–18 px label.

### Portrait composition

- Bust or shoulders-up, 4:5 crop, face and identifying prop inside a 10% safe
  margin; transparent foreground preferred for cards.
- Head angle is three-quarter or front, with eye line aligned across the cast.
- Neutral background: transparent cutout plus optional 512 x 640 case-file
  background plate. Do not bake UI text into the art.
- Master: layered 2048 x 2560 source. Runtime: 512 x 640 RGBA PNG; provide a
  256 x 320 derivative for dense lists.
- Lighting: one warm key and one cool rim. Injured/exhausted states may alter
  light and posture, but must preserve identity.
- Wardrobe should signal trade: picks and gloves, heavy coat, immaculate suit,
  tool belt, radio headset, camera, or electrician's kit. Avoid firearms as the
  default identity cue.

### Portrait naming

```text
assets/images/portraits/<character_id>_<state>.png
assets/images/portraits/class_<class_id>_badge.png
assets/images/portraits/portrait_unknown.png
assets/images/portraits/portrait_locked.png
```

## 2. Target, district, and safehouse art

Targets are cards and procedural floorplans, not explorable levels. Art should
sell the mark and the city while leaving the encounter graph to code.

- [ ] One location thumbnail per target archetype: casino, warehouse, mansion,
  bank, gallery, hotel, nightclub, office, dock, and any additional target
  categories present in `assets/data/targets.json`.
- [ ] Three environment plates: night/clear, night/rain, and night/fog. These
  are atmospheric backplates for the Board, Planning, and Results screens, not
  gameplay maps.
- [ ] Safehouse desk plate: case folders, brass lamp, radio, coffee, lockbox,
  and a clean negative-space region for Crew/Outfitter cards.
- [ ] The Board needs a city-map texture or paper map only if the screen uses
  a map composition. Keep the target list usable without it.
- [ ] Target thumbnails: 640 x 360 RGBA/opaque PNG, 16:9, focal subject on the
  right or left according to the screen layout, with a text-safe zone.
- [ ] Environment plates: 1920 x 1080 JPG/PNG for opaque art; provide a darkened
  1280 x 720 derivative for UI overlays.
- [ ] Do not paint encounter doors or node positions into a target thumbnail;
  those belong to the procedural floorplan.

## 3. Equipment and loot icons

`assets/data/equipment.json` contains 67 equipment templates across five slots
and five rarity tiers. Every item shown in the Outfitter, Crew, Results, or loot
summary needs a stable visual key.

### Slot families

- [ ] Tool: picks, tension sets, drill, bypass kit, climbing kit.
- [ ] Weapon/defence: non-glamorized emergency tools, baton, vest, restraint
  kit, or the actual authored items in the data.
- [ ] Outfit/disguise: coat, uniform, mask, gloves, credentials, shoes.
- [ ] Tech: radio, camera loop, signal jammer, laptop, access device.
- [ ] Utility: medkit, rope, light, bag, decoy, consumable.

### Icon requirements

- 67 named item icons, plus `loot_unknown`, `loot_cash`, `loot_document`, and
  `loot_jewellery` if those result categories are displayed.
- 64 x 64 RGBA PNG delivery; 128 x 128 source/master. Keep the object centered
  inside a 52 x 52 safe area with a consistent three-quarter view.
- Five rarity treatments: basic, improved, advanced, rare, legendary. Prefer a
  shared object drawing with procedural rarity frame/foil so variants do not
  multiply the art burden.
- State overlays: equipped, selected, locked/level-gated, worn, broken, sold,
  and newly found. Use a separate overlay or code-drawn badge where possible.
- No baked item names, stats, rarity labels, or prices in the bitmap.
- Strong silhouette at 24 x 24 and recognizable in the 44 x 44 touch target.

### Naming and manifest

```text
assets/images/items/<equipment_id>.png
assets/images/items/loot_<kind>.png
assets/images/items/slot_<slot>.png
```

The item manifest must match `equipment.json` one-for-one. Add a validation
test that reports missing or duplicate keys before a publish.

## 4. Floorplan, door, and encounter artwork

The floorplan is the signature visual: a clean case-file blueprint that turns a
sequence of encounters into a readable building.

- [ ] Procedural node symbols for every encounter trade: social, lockpicking,
  stealth, hacking, athletics, combat, investigation, and any authored type.
- [ ] A bespoke symbol for each environment complication: crowded, noisy, dark,
  exposed, wet, watched, alarmed, and any `environment.json` factor.
- [ ] Node states: unknown, cased, selected, assigned, ready, in progress,
  success, failure, critical success, critical failure, skipped, and locked.
- [ ] Door/room silhouettes: lobby, service door, vault, office, loading dock,
  roof, alley, elevator, and exit. Draw in the toolkit `paint`/UI layer unless
  a screen proves a bitmap is needed.
- [ ] Connecting route lines should look like red pencil or cyan drafting ink;
  they must remain visible under node highlights.
- [ ] Assigned crew portrait medallion at each node; do not use a tiny full
  portrait that becomes unrecognizable.
- [ ] A small target-specific floorplan legend with labels, never icon-only.

If any floorplan texture is commissioned, use a seamless 1024 x 1024 paper or
blueprint texture with low contrast and transparent/neutral margins. No fixed
door positions may be baked into it.

## 5. Dice, resolution, and heist VFX

The d20 resolution sequence is the emotional payoff. Its art should be crisp,
fast, and inspectable rather than particle-heavy.

- [ ] d20 face treatment: ivory/black base, brass edge, cyan critical glow,
  red critical-failure crack. Faces 1–20 must remain readable at 32 px.
- [ ] Modifier chips for attribute, skill, equipment, condition, environment,
  and chemistry. Each gets a glyph and a short label.
- [ ] Door transition VFX: paper snap, brass stamp, cyan scan sweep, red alarm
  pulse, and a restrained dust/rain of paper flecks.
- [ ] Outcome marks: cleared, partial/neutral, failed, critical success, and
  critical failure. Each state needs icon + label + motion/shape difference.
- [ ] Alarm/heat treatment: rotating beacon reflection or pulsing red edge,
  never a full-screen flash that harms readability.
- [ ] Injury/fatigue treatment: bandage/clock glyph and subdued portrait change;
  avoid blood as the primary signal.
- [ ] Loot reveal: folder slide, evidence stamp, or velvet-lined reveal frame.

All effects must support reduced motion and low-opacity modes. Do not make
procedural VFX depend on external bitmap sprites unless a visual test proves
the effect cannot be drawn by the toolkit.

## 6. Global UI chrome and iconography

- [ ] Master Thief wordmark: horizontal title lockup, compact monogram, and
  monochrome light/dark versions. Runtime title art: 640 x 160 transparent PNG.
- [ ] Tab icons for Crew, The Board, Outfitter, Last Job, and Records.
- [ ] Save, load, delete, settings, sound, music, fullscreen, help, close,
  back, warning, info, success, danger, search, filter, sort, add, remove,
  lock, unlock, and notification glyphs.
- [ ] Heist-specific glyphs: case file, target, payout, reputation, notoriety,
  heat, safehouse, payroll, doctor, fence, bail, retire, door, dice, and crew.
- [ ] 24 x 24 design grid; deliver 48 x 48 RGBA PNG masters. Maintain one stroke
  weight and one corner language across the family.
- [ ] Icon buttons keep visible text labels for unfamiliar actions and retain a
  minimum 44 x 44 logical-pixel touch target.
- [ ] Plaque/button chrome remains procedural. Bitmap frames are optional and
  must not replace the toolkit's responsive hit-tested surfaces.

## 7. Records, achievements, and campaign marks

- [ ] Achievement badge family for the five authored achievement categories.
- [ ] Campaign record stamps: first job, clean job, botched job, legendary loot,
  retired outfit, heat peak, and long-running crew.
- [ ] Three tiers per badge (locked, earned, notable) using silhouette/foil,
  not color alone.
- [ ] 96 x 96 RGBA PNG badge masters, delivered at 48 x 48 and 24 x 24.
- [ ] No badge may imply a mechanic that does not exist in `achievements.json`.

## 8. Production rules and technical delivery

### Formats and folders

```text
assets/images/
  brand/
  portraits/
  targets/
  environments/
  items/
  icons/
  achievements/
  effects/
  ui/
source_art/                 # layered/editable masters; not loaded at runtime
assets/artwork_manifest.json
```

- PNG RGBA for portraits, sprites, icons, logos, and transparent UI art.
- JPG/PNG for opaque environment plates; use PNG when crisp type or linework is
  embedded in the image (normally avoid embedded type).
- Nearest filter for pixel/flat icon art; linear filter for painted portraits,
  plates, and soft effects. Declare the choice in `artwork_manifest.json`.
- Lowercase `snake_case`; semantic identity first, state last:
  `vera_sloan_injured.png`, `icon_heat_high.png`, `target_velvet_room.png`.
- Do not put prices, DCs, names, screen positions, or revision numbers in
  runtime filenames.

### Accessibility and readability

- [ ] Every state that matters to a decision is expressed by at least two of
  icon, label, shape, pattern, position, or motion.
- [ ] Check all critical text and glyphs against `#0B1018`, `#162331`, and
  `#E7D8B7` surfaces.
- [ ] Verify at 1280 x 720, 1024 x 576, and a narrow 800 x 600 window.
- [ ] Portraits and target thumbnails must not obscure the candidate's stats or
  the target's payout/required reputation.
- [ ] All required actions remain possible through visible touch/click targets;
  keyboard shortcuts are supplemental.
- [ ] Provide reduced-motion behavior for dice, scan, alarm, and loot reveals.

### Copyright and provenance

- [ ] Record artist/generator, prompt or source, edit history, date, and license
  beside each shipped asset family.
- [ ] No real bank logos, police insignia, branded weapons, copyrighted movie
  characters, or recognizable celebrity likenesses.
- [ ] Generated images must be reviewed for accidental text, watermarks, extra
  fingers, broken props, unreadable dice, and inconsistent character identity.

## 9. Asset manifest and validation

Create `assets/artwork_manifest.json` with one entry per runtime texture:

```json
{
  "textures": [
    {
      "key": "portrait_vera_sloan_neutral",
      "path": "assets/images/portraits/vera_sloan_neutral.png",
      "filter": "linear"
    },
    {
      "key": "icon_heat_high",
      "path": "assets/images/icons/icon_heat_high.png",
      "filter": "nearest"
    }
  ]
}
```

- [ ] Add a test that every character ID, equipment ID, target ID, achievement
  ID, and icon key used by the UI resolves to exactly one asset or an explicit
  procedural renderer.
- [ ] Add alpha-edge and dimensions checks for PNG deliveries.
- [ ] Add a visual verification capture for each screen listed above. Replace
  an existing capture of the same state rather than duplicating it.
- [ ] Run `.\publish.ps1` from the project root after each meaningful art batch.

## Definition of done

The art pass is complete when:

- [ ] Every authored character, equipment item, target category, achievement,
  and visible semantic state has either a named asset or a documented
  procedural renderer.
- [ ] All eight primary screens and modal/settings states have a coherent noir
  payday presentation with no placeholder art.
- [ ] Crew identity remains readable in list, detail, planning-node, run, and
  results contexts.
- [ ] Equipment and loot remain recognizable at the smallest rendered size.
- [ ] The floorplan and dice sequence remain the visual focus of Planning and
  The Run without obscuring the numbers that drive decisions.
- [ ] The manifest, asset-key test, screenshots, and `publish.ps1` all pass.
- [ ] The final catalog thumbnail shows the title-screen noir safehouse/board
  composition and uses the same palette and wordmark.

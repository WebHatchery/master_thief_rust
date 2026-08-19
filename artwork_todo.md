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


# Pixel-Style Game Genres Suitable for Agent Development

A survey of the Cocos Creator game library at `/Users/ryan/vibe/cocos-dev-games`
(117 projects), cross-referenced with the existing dependency-free pixel samples
(`samples/pixel-roguelike`, `samples/void-descent`, and siblings), to identify
which game genres can be rebuilt in a **single unified pixel-art style** with
**simplified art but fully preserved gameplay** — and which of those are good
candidates for an agent to develop end to end.

---

## 1. Background

### 1.1 The Cocos library

The `cocos-dev-games` directory holds 117 Cocos Creator projects. Most belong to
two large studio series (`ogg-*` and `tcg*`). Grouped by genre:

| Genre cluster | Count | Examples |
|---|---|---|
| Shooter (top-down / arena / tank) | 10 | `PandaShoot`, `ogg-2p-shoot`, `gunman`, `tankgo` |
| Roguelike / Survivor | 9 | `tcg-act-rogue`, `ogg-survivor`, `ogg-egg-roguelike`, `ogg-stick-rouge` |
| Simulation / Manager | 9 | `ogg-town`, `catmarket`, `grandma_iland`, `win_one_city` |
| Mini-game / skill | 9 | `tcgbowl`, `tcgbird`, `tcg-catch-pig`, `tcgcleanfish` |
| Tower Defense | 8 | `KindomRushTS`, `ogg-zombie-td`, `tcgtdsimple`, `tcg-eat-bag-td` |
| Platformer / Action | 8 | `ogg-pix-platform`, `ogg-stick-ninja`, `lightsword`, `doom_run` |
| Casual / Hypercasual | 8 | `ballgame`, `pixel_fish`, `ogg-super-fish`, `pixo` |
| Idle / Incremental | 7 | `idlebusiness`, `ogg-idle-sg`, `stackland`, `ogg-lootbox` |
| Block-shooter / Brick breaker | 6 | `ogg-block-shooter`, `superbrick`, `boxclean`, `cleanup` |
| Adventure / RPG | 6 | `ogg-hero-adv`, `tcggoldhunter`, `textlife`, `tcgadv` |
| Battle / Fighting | 6 | `tcgbattleall`, `tcgswordbattle`, `ogg-tiny-battle` |
| Puzzle / brain | 6 | `ogg-ball-puzzle`, `tcg2048`, `tcg-iron-dragon` |
| Merge / Combination | 5 | `tcgslimego`, `ogg-legend`, `tcgjpcard`, `tcgsortcard` |
| 3D / Action-3D | 4 | `ogg-car-3d`, `ogg-evowars`, `tcgmy3d`, `bladesword` |
| Match-3 | 3 | `demo_三消_消消乐`, `tcgcandy`, `card-chess` |
| Manager / building | 3 | `末日特工队`, `wow_restore`, `狩猎高手` |
| Frameworks / misc | 5 | `runtime`, `tcg-u-come`, `sheep` |

### 1.2 The pixel samples

The samples in `samples/` already prove a reusable production pattern:

- **Dependency-free HTML5 Canvas** — no engine, no build step, just `index.html`
  + modular plain JS.
- **Procedural pixel art** — sprites are editable integer matrices mapped to a
  shared palette (`pixel-roguelike/js/assets.js`, `void-descent/js/art.js`).
  No external image assets, no spritesheets, no skeletal rigs.
- **Compact, modular code** — each game is ~500–3500 LOC split into clear
  modules (`constants` / `assets` / `renderer` / `game` / `i18n` / per-entity
  files).
- **Responsive desktop + mobile** with touch joystick/buttons and safe-area
  layout, plus `localStorage` save.

Existing samples and the genres they already validate:

| Sample | Genre proven | LOC |
|---|---|---|
| `pixel-roguelike` | Top-down action roguelike | ~3400 |
| `void-descent` | Turn-based dungeon crawler | ~2000 |
| `pixel-backpack-roguelite` | Inventory/auto-battler roguelite | ~1300 |
| `pixel-platformer-infinite` | Procedural side-scroll platformer | ~730 |
| `pixel-merge-garden` | Merge-idle | ~260 |
| `pixel-idle-forge` | Idle/incremental | ~530 |
| `pixel-fishing-idle` | Timing + idle collection | — |
| `pixel-mart-manager` | Shop/management sim | — |
| `canvas-tower-defense` | Tower defense | — |
| `canvas-match3-puzzle` | Match-3 | — |
| `canvas-brick-breaker` | Brick breaker | — |
| `pixel-tower-defense` | Tower defense (upgrade trees, 3 maps) | ~1540 |
| `pixel-card-spire` | Roguelike deck-builder | ~1690 |
| `pixel-survivors` | Wave-survivor roguelite | ~1370 |
| `pixel-town-tycoon` | Grid management / supply-chain sim | ~900 |
| `pixel-match-quest` | Objective match-3 with obstacles | ~1150 |
| `pixel-arcade` | 5-game mini-game pack | ~720 |

---

## 2. What makes a genre a good fit

A genre rebuilds well in a unified pixel style **and** is tractable for an agent
when it scores high on these axes:

**Pixel-art friendliness**
- Gameplay depth comes from **rules and systems**, not art fidelity.
- Entities are **small and discrete** — representable as 8×8 / 16×16 matrices
  or single glyphs.
- Camera is **top-down, grid-based, single-screen, or simple side-scroll** —
  no perspective, no 3D meshes.
- **Minimal animation** — state changes read fine as palette swaps, 2–4 frame
  loops, or tweened transforms; no skeletal rigs or hand-keyed character art.

**Agent-development friendliness**
- **Procedural content** — levels/waves/loot generated from seeds, so no
  hand-authored asset pipeline is required.
- **Deterministic, testable systems** — combat, economy, and progression can be
  unit-reasoned and verified headlessly.
- **Self-contained** — no netcode, no real-time multiplayer, no server.
- **Bounded scope** — a satisfying vertical slice fits in ~500–3500 LOC, the
  proven range of the existing samples.

**Anti-patterns (low fit)**
- True 3D (`ogg-car-3d`, `tcgmy3d`, `bladesword`, `shoot3d`).
- Real-time PvP / MOBA (`ogg-wz-lol`, `ogg-2p-shoot`) — netcode and balance.
- Animation-driven action fighting (`ogg-kongfu`, `tcgswordbattle`) — depth
  lives in frame data and hitboxes that pixel simplification erodes.
- Physics-tuning-heavy sports (`ogg-football`, `tcgbowl`) — feel depends on
  fine-tuned physics, not systems.
- Content-heavy narrative (`textlife`) — art-light but writing-bound, not a
  systems problem.

---

## 3. Genre suitability tiers

### Tier A — Highly recommended (build these first)

| Genre | Cocos analogues | Why pixel works | Why agent works | Reference |
|---|---|---|---|---|
| **Action roguelike / survivor** | `tcg-act-rogue`, `ogg-survivor`, `ogg-stick-rouge`, `ogg-egg-roguelike` | Top-down, tiny sprites, particle combat | Procedural waves + skill cards; pure systems depth | `pixel-roguelike` |
| **Tower defense** | `KindomRushTS`, `ogg-zombie-td`, `tcgtdsimple`, `tcg-eat-bag-td` | Grid map, static towers, glyph creeps | Wave tables, pathfinding, tower stats — all data-driven | `canvas-tower-defense` |
| **Idle / incremental** | `idlebusiness`, `ogg-idle-sg`, `ogg-lootbox` | Almost no animation; UI-driven | Pure number systems, prestige curves, offline earnings | `pixel-idle-forge` |
| **Merge** | `tcgslimego`, `ogg-legend`, `tcgjpcard` | Grid of identical-tier sprites | Merge rules + order/mutation systems are trivial logic | `pixel-merge-garden` |
| **Match-3 / tile elimination** | `demo_三消_消消乐`, `tcgcandy`, `sheep` | Grid of colored glyphs | Match detection, cascades, objectives — classic deterministic logic | `canvas-match3-puzzle` |
| **Backpack / auto-battler roguelite** | `lineup`, `stackland` | Inventory grid + auto-resolved fights | Spatial planning + autobattle = systems, not reflexes | `pixel-backpack-roguelite` |
| **Turn-based dungeon crawler** | `tcg-mouse`, `tcg-space-ghost`, `majo` | Tile grid, fog of war, glyph monsters | Turn logic is fully deterministic and testable | `void-descent` |

### Tier B — Recommended with caveats

| Genre | Cocos analogues | Caveat | Reference |
|---|---|---|---|
| **Brick breaker / block shooter** | `superbrick`, `ogg-block-shooter`, `ogg-tapblox` | Needs solid ball physics, but well-bounded | `canvas-brick-breaker` |
| **Infinite procedural platformer** | `ogg-pix-platform`, `ogg-stick-ninja` | Jump feel needs tuning; keep it auto-generated, not hand-authored | `pixel-platformer-infinite` |
| **Shop / town management sim** | `catmarket`, `ogg-town`, `grandma_iland` | Economy balancing is iterative; scope creep risk | `pixel-mart-manager` |
| **Number / sliding puzzle** | `tcg2048`, `tcg-iron-dragon` | Trivial to build; depth ceiling is low | — |
| **Timing / fishing collection** | `pixel_fish`, `ogg-super-fish`, `tcgcleanfish` | Single timing mechanic; pad with collection meta | `pixel-fishing-idle` |
| **Top-down twin-stick shooter** | `PandaShoot`, `gunman`, `rushshoot`, `tankgo` | Fun, but enemy/bullet variety drives the work | `pixel-roguelike` (combat) |
| **Turn-based card battler** | `tcgact`, `card-chess`, `tcg-fire-strong` | Card-effect engine is the hard part; very rewarding | `pixel-backpack-roguelite` |
| **Catch / cleanup mini-games** | `tcg-catch-pig`, `boxclean`, `cleanup`, `tcgmanclean` | Easy wins, but shallow — good for quick samples | — |

### Tier C — Not recommended for pixel + agent rebuild

| Genre | Cocos analogues | Reason |
|---|---|---|
| 3D driving / 3D action | `ogg-car-3d`, `tcgmy3d`, `bladesword`, `shoot3d` | Requires 3D rendering; pixel style does not apply |
| MOBA / real-time PvP | `ogg-wz-lol`, `ogg-2p-shoot`, `ogg-evowars` | Netcode + live balance out of scope |
| Animation-driven fighting | `ogg-kongfu`, `tcgswordbattle`, `ogg-ninja` | Depth lives in frame data lost to simplification |
| Physics-feel sports | `ogg-football`, `tcgbowl` | Quality = physics tuning, not systems |
| Narrative adventure | `textlife`, `tcgadv` | Bound by writing/content volume, not code |

---

## 4. Build list for agent development — **implemented**

The six genres below were each built end to end as complete, dependency-free
samples in the planned order. Every game ships with full screens, mobile +
multi-resolution layout, `localStorage` save, English/中文, and was verified
with a headless simulation harness.

| # | Sample | Genre | Cocos analogues | JS LOC |
|---|---|---|---|---|
| 1 | `samples/pixel-tower-defense` | Tower defense — 4 tower families with branching tier-3 upgrades, 7 enemy types, 3 maps, endless mode | `KindomRushTS`, `ogg-zombie-td`, `tcgtdsimple` | ~1540 |
| 2 | `samples/pixel-card-spire` | Roguelike deck-builder — data-driven card engine, 26 cards, relics, 12-floor branching map, boss | `tcgact`, `card-chess`, deck `tcg*` | ~1690 |
| 3 | `samples/pixel-survivors` | Wave-survivor roguelite — 6 evolving weapons, level-up draft, bosses, meta-progression | `ogg-survivor`, `tcgvirus` | ~1370 |
| 4 | `samples/pixel-town-tycoon` | Grid management sim — supply chains, adjacency bonuses, worker economy, ranks, quests, offline | `ogg-town`, `catmarket`, `idlebusiness` | ~900 |
| 5 | `samples/pixel-match-quest` | Objective match-3 — specials, ice/crate/drop obstacles, 12 levels, boosters, map | `demo_三消_消消乐`, `tcgcandy` | ~1150 |
| 6 | `samples/pixel-arcade` | Mini-game pack — 5 skill games (flap / catch / reflex / stack / dash) in one shell, medals | the 9 `tcg*` mini-games | ~720 |

Each reuses the shared foundation below.

---

## 5. Shared technical foundation

Every agent-built pixel game should reuse the proven sample architecture rather
than reinvent it:

- **Stack**: single `index.html` + modular plain JS, no dependencies, no build.
- **Art**: integer-matrix sprites + a shared palette (see
  `void-descent/js/art.js`). Simplify freely — readability over fidelity.
- **Module layout**: `constants` / `assets` / `renderer` / per-entity files /
  `game` (loop + state) / `i18n` (English default, Chinese toggle).
- **Platform**: responsive 16:9 desktop and full-screen mobile with touch
  controls and safe-area padding.
- **Persistence**: `localStorage` autosave; offline progress where the genre
  calls for it.
- **Scope discipline**: target the 500–3500 LOC band that the current samples
  occupy — enough for a satisfying vertical slice, small enough to verify.

---

## 6. Summary

Of the 117 Cocos projects, roughly **70%** fall into genres (roguelike, tower
defense, idle, merge, match-3, auto-battler, dungeon crawler, management) that
rebuild cleanly in a unified pixel style with simplified art and **no loss of
gameplay** — because their depth is systemic, not artistic. The remaining ~30%
(3D, MOBA, animation-driven fighting, physics sports, narrative) depend on
fidelity or infrastructure that pixel simplification cannot preserve and an
agent cannot cheaply build.

The highest-value next targets for agent development are **turn-based card
battlers**, **tower defense variants**, and **wave-survivor roguelites** — each
high in gameplay depth, low in art dependency, and a natural extension of the
existing pixel sample family.

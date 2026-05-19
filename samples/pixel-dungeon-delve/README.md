# Pixel Dungeon Delve

A dependency-free, turn-based roguelike crawler — every step is a turn. Descend
procedurally generated floors, fight by bumping foes, and delve as deep as you
dare. Unlike the real-time `pixel-roguelike` sample, this is a classic
deliberate, grid-based dungeon crawl.

## Features

- Procedurally generated floors: rooms, corridors, stairs, fully fresh each run.
- Fog of war with raycast field-of-view — explored tiles stay dimly remembered.
- Turn-based bump combat: move into a foe to attack, enemies act in response.
- 6 enemy types gated by depth (rats, gloom bats, skeletons, orc brutes,
  wraiths) up to the Elder Wyrm boss on depth 10.
- Loot: health potions, gold, and 5 tiers each of weapons and armor that
  auto-equip when stronger.
- Level-ups grow max HP, attack, and defense and fully heal you.
- Depth-scaled difficulty and an endless descent past the boss for score chasing.
- `localStorage` run save (resume mid-delve) plus a best-depth record.
- English / 中文 toggle.
- Responsive square canvas: keyboard (arrows / WASD), on-screen d-pad, and
  swipe-to-move on touch devices.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4190
```

Then visit `http://127.0.0.1:4190/index.html`.

## Play

- **Desktop:** arrow keys / WASD to move, `Space` or `.` to wait, `Q` to drink a
  potion, `Esc` to pause.
- **Mobile:** swipe on the dungeon or use the d-pad; tap the centre to wait.
- Bump an enemy to attack it. Plan around line of sight — foes that can't see
  you won't chase.
- Step onto the stairs to descend. Drink potions before a fight, not after.
- Slay the Elder Wyrm on depth 10, then keep delving for a deeper record.

## Structure

- `index.html` - title / game screens, HUD, controls, overlays.
- `css/style.css` - responsive dungeon UI, HUD bars, d-pad.
- `js/i18n.js` - English / Chinese strings.
- `js/data.js` - map constants, enemies, gear, tuning curves.
- `js/art.js` - 16x16 pixel sprites for terrain, heroes, enemies, items.
- `js/game.js` - floor generation, FOV, turn loop, combat, save, screen flow.

# Pixel Survivors

A dependency-free HTML Canvas wave-survivor roguelite — move to dodge while your
weapons fire automatically.

## Features

- 13 weapons (dagger / holy aura / orbit blade / chain bolt / frost nova /
  fireball / shard burst / boomerang / arc coil / skyfall / holy lance /
  trident / spike mine), each with 6 levels and an evolved final form.
- 6 stackable passive items (might / swift / haste / armor / magnet / vitality).
- Level-up draft: choose 1 of 3 upgrades each level.
- 6 enemy types with time-scaled hordes plus 3 escalating bosses, ending in a
  15-minute Overlord finale.
- XP gems, gold coins, hearts, and boss treasure chests with sticky magnet pull.
- Meta-progression: 7 permanent upgrades bought between runs with banked gold.
- Frost slow, knockback, contact i-frames, and a Second Wind revive.
- `localStorage` save, English/中文 toggle.
- Responsive desktop + mobile: keyboard or floating touch joystick, camera that
  follows the survivor.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4183
```

Then visit `http://127.0.0.1:4183/index.html`.

## Play

- **Desktop:** WASD / arrow keys to move. **Mobile:** touch and drag anywhere.
- Weapons fire on their own — focus on dodging and collecting gems.
- Each level-up offers three upgrades; build toward weapon evolutions.
- Survive to 15:00 and slay the Overlord.

## Structure

- `index.html` - title / armory / game screens and overlays.
- `css/style.css` - responsive pixel UI, HUD, level-up cards, joystick.
- `js/i18n.js` - English/Chinese strings.
- `js/data.js` - weapons, passives, enemies, bosses, meta upgrades.
- `js/art.js` - top-down pixel sprites, terrain, and pickups.
- `js/game.js` - engine, weapons, spawning, level-up, meta, screen flow.

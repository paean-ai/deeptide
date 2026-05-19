# Pixel Deep Miner

A dependency-free dig-down mining game — drill through procedural earth for ore,
manage fuel and cargo, dodge lava, and fly back to the surface to sell and
upgrade your rig. A fresh genre alongside the other `samples/` pixel games.

## Features

- Procedurally generated 220-tile-deep world: dirt, stone, hard rock, lava
  pockets and bedrock, with depth-gated ore veins (copper → iron → silver →
  gold → gem).
- Smooth grid physics: walk, drill, gravity-driven falls with fall damage, and
  thruster flight back up.
- Resource loop: fuel drains underground, cargo fills as you mine — surface to
  refuel, repair, sell ore, and buy upgrades.
- 5 upgrade tracks (Drill, Fuel Tank, Cargo Hold, Hull, Thruster), each 6 tiers.
- Hazards: lava scorches the hull (mitigated by Hull tier), long falls hurt,
  running dry of fuel strands the rig.
- Particle bursts on every dig, animated drill, glowing ore.
- `localStorage` run save (resume mid-dig) and a best-depth record.
- English / 中文 toggle.
- Responsive canvas: keyboard (arrows / WASD — hold to keep digging), on-screen
  d-pad, and swipe-to-dig on touch devices.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4191
```

Then visit `http://127.0.0.1:4191/index.html`.

## Play

- **Desktop:** hold arrow keys / WASD to dig in a direction; `Esc` pauses.
- **Mobile:** hold the d-pad or swipe across the world.
- Dig **down** to reach richer ore; press **up** to fly (burns thruster fuel).
- Watch the fuel bar — keep enough to climb back. The surface refuels and
  repairs you for free and opens the shop.
- Sell ore, upgrade your rig, and chase a deeper record.

## Structure

- `index.html` - title / game screens, HUD, d-pad, shop & overlays.
- `css/style.css` - responsive mining UI, HUD bars, shop.
- `js/i18n.js` - English / Chinese strings.
- `js/data.js` - world constants, blocks, ores, upgrade tracks, tuning.
- `js/art.js` - pixel terrain blocks, the mining rig, the shop hut.
- `js/game.js` - world gen, physics, drilling, fuel/cargo, shop, save, loop.

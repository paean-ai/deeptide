# Pixel Fishing Idle

A dependency-free fishing idle game — cast a line, stop the marker in the green
band to land a catch, fill orders, sail to new waters, and grow a crew that
hauls fish while you're away.

## Features

- Delta-timed catch minigame: a marker sweeps a bar — stop it in the green band
  for a catch, dead-centre for a **perfect** (x1.6) haul.
- 20 fish across 5 zones (Cove → Kelp Bay → Moon Reef → Sunken Crown →
  Frost Tides), each drawn as a distinct pixel species (small, round,
  long, flat, jelly, crab, serpent silhouettes) with 5 rarity tiers.
- Ambient fish swimming below the waterline, splash particles, weather (Calm /
  Storm / Lucky Tide) that shifts the odds.
- Rod / Bait / Boat / Crew upgrades — rod widens the catch band, bait surfaces
  rarer fish, crew hauls fish on a timer.
- Order board, a collection log, and zone unlocks gated by boat level.
- `localStorage` save with capped **offline crew income** for up to 8 hours.
- English / 中文 toggle.
- Responsive canvas (keeps aspect on desktop, fills the screen on mobile);
  tap / click / space to cast and reel.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4180
```

Then visit `http://127.0.0.1:4180/index.html`.

## Play

- Press **Cast Line** (or tap the water / press space) to cast.
- Wait for the bite, then **Reel In** when the sweeping marker is inside the
  green band — centre it for a perfect catch.
- Spend coins on Rod, Bait, Boat, and Crew; sail to richer zones once your boat
  is strong enough.
- Crew members keep hauling fish on their own — even while the tab is closed.

## Structure

- `index.html` - canvas, HUD, controls, collection log.
- `css/style.css` - responsive layout, HUD, controls.
- `js/data.js` - zones, rarities, fish roster, upgrade tuning.
- `js/i18n.js` - English / Chinese strings and fish names.
- `js/art.js` - pixel fish species sprites, boat, lure shadow.
- `js/game.js` - catch minigame, economy, idle/offline income, save, loop.

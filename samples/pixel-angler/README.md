# Pixel Angler

A pixel-art fishing game built around an active reeling minigame. Cast your
line, hook the fish the instant it bites, then hold steady to reel it in. A
fresh skill-and-progression fishing genre — distinct from the idle
`pixel-fishing-idle` sample.

## Features

- A three-step catch loop: cast → a reflex tap to set the hook → a
  hold-to-reel minigame.
- The reeling minigame — a catch bar rises while you hold and falls when you
  let go; keep it over the darting fish to fill the catch gauge before it
  drains away.
- 9 fish species across three waters tiers; bigger, rarer fish dart harder.
- A tackle shop — upgrade the **rod** for a wider catch bar, the **reel** for
  a faster fill, and unlock deeper waters with richer fish.
- Persistent progression — coins, upgrades, unlocked waters and a per-species
  catch log, all saved to `localStorage`.
- English / 中文 toggle.
- Responsive 360:480 scene — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4233
```

Then visit `http://127.0.0.1:4233/index.html`.

## Play

- Tap CAST to throw your line, then wait for a bite.
- The moment the line reads **BITE!**, tap the water to set the hook.
- Hold anywhere to raise the green catch bar, release to let it drop — keep
  the fish inside it until the gauge fills.
- Sell your catch for coins and visit the SHOP to upgrade your gear and reach
  deeper waters.

## Structure

- `index.html` - shell, title / game / shop screens.
- `css/style.css` - responsive 360:480 shell, HUD, shop list.
- `js/data.js` - fish species, waters tiers and the upgrade economy.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - sky / water scene, angler, bobber and fish sprites.
- `js/game.js` - the cast/hook/reel loop, reeling minigame, shop, save.

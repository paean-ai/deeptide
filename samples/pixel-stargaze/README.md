# Pixel Stargaze

A pixel-art **idle astronomy game**. Tap the night sky to gather Light by
hand, build telescopes that gather it on their own — even while you are away —
and publish papers to reset for Renown, a permanent boost. A fresh idle genre
alongside the other `samples/` pixel games.

## Features

- Five telescope tiers — Lens, Reflector, Astrograph, Radio Dish, Space Scope
  — each gathering Light passively; prices climb the classic ×1.15 ladder so
  there is always a next goal.
- Eight one-time research upgrades that multiply a tier, all output, or your
  tap, layering into a steady exponential climb.
- A Publish-a-paper prestige loop: bank a run for Renown, where every point of
  Renown adds +15% to all output forever.
- Offline accrual: telescopes keep gathering while the tab is closed, capped
  at eight hours, with a "while you were away" summary on return.
- Twinkling-starfield art, compact K/M/B number formatting, English/中文
  toggle, autosaved to `localStorage`.
- Verified: 46 checks — tapping, the rate formula, telescope pricing, research
  multipliers (tier / global / tap), publishing and the Renown multiplier,
  capped offline accrual; an economy sim confirms a 15-minute run can publish
  and a two-hour prestige run never overflows; plus a 4-script smoke test that
  collects an offline popup, taps, buys and switches tabs.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4445
```

Then visit `http://127.0.0.1:4445/index.html`.

## Play

- Tap the night sky to gather Light by hand.
- Spend Light on telescopes — they gather Light on their own, around the
  clock, and keep going while you are away.
- Buy research upgrades to multiply your output.
- When a run has gathered enough, tap **Publish** to reset it for Renown — a
  permanent multiplier that makes every future run faster.

## Structure

- `index.html` - shell, single canvas, four script tags.
- `css/style.css` - responsive 360:480 portrait shell.
- `js/data.js` - the idle economy: telescopes, research, prestige, offline.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - the night sky, the shop list, number formatting, title art.
- `js/game.js` - screen flow, the idle loop, input, save + offline accrual.

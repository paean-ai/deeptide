# Pixel Quest

A pixel-art **turn-based JRPG party battler**. Lead three heroes — a Knight, a
Mage and a Cleric — through a campaign of escalating encounters, choosing each
action turn by turn. A fresh classic-RPG-combat genre alongside the other
`samples/` pixel games.

## Features

- An 8-stage campaign from a slime hollow to a dragon's lair.
- Three heroes, each with a distinct skill: the Knight's **Cleave** strikes
  every foe, the Mage's **Firestorm** burns them all with magic, the Cleric's
  **Mend** restores an ally's HP.
- Turn order runs on speed; on a hero's turn pick Attack, their Skill (spends
  MP), or Defend to halve the next hit.
- Heroes grow stronger every stage, and the party is fully restored at the
  start of each battle — so every stage is a fair, self-contained fight.
- All eight stages are verified winnable by an automated playthrough.
- Level select with progressive unlocks and per-stage completion marks, saved
  to `localStorage`.
- English / 中文 toggle.
- Responsive 360:480 board — scales on desktop, fills the screen on mobile.

## Run

Open `index.html` directly, or serve the folder:

```bash
python3 -m http.server 4250
```

Then visit `http://127.0.0.1:4250/index.html`.

## Play

- When a hero's marker lights up, tap ATTACK then a foe, tap SKILL to use their
  ability, or tap DEFEND to brace.
- Skills cost MP — Cleave and Firestorm hit every enemy at once, Mend heals the
  ally you pick.
- Defeat every enemy to win the stage; if all three heroes fall, the stage is
  lost.

## Structure

- `index.html` - shell, title / battle-select / game screens, result overlay.
- `css/style.css` - responsive 360:480 shell, HUD.
- `js/data.js` - heroes, skills, foes, stages and the turn-based combat rules.
- `js/i18n.js` - English / Chinese strings.
- `js/art.js` - battle layout, unit sprites, HP/MP bars, action menu.
- `js/game.js` - the battle UI, turn pacing, save.

# Roguelike And Roguelite Game Template

Use this skill when implementing a roguelike, roguelite, dungeon crawler,
survivor-like, or backpack roguelite sample.

## Goal

Create a complete loop with procedural variation, meaningful progression,
readable combat, and replayable decisions. Avoid half-built mechanics that do
not connect to the core loop.

## Choose The Subgenre

Pick one primary loop before writing code:

- **Action roguelite:** real-time movement, enemy waves, skills, XP, bosses.
- **Turn-based dungeon crawler:** rooms, corridors, fog, turns, items, floor
  descent.
- **Backpack roguelite:** prep phase, spatial inventory, merge/forge decisions,
  autonomous combat, rewards.
- **Platform roguelite:** procedural traversal, upgrades, enemies, score chase.

Do not mix every pattern at once. The sample should have one dominant decision
surface.

## Core Loop Examples

Action roguelite:

```text
explore/move -> fight wave -> collect XP -> choose skill -> stronger wave -> boss
```

Turn-based dungeon:

```text
enter floor -> reveal rooms -> fight/loot -> find exit -> choose upgrade -> descend
```

Backpack roguelite:

```text
shop -> arrange/merge/forge backpack -> auto battle -> reward/upgrade -> shop
```

## Required Systems

- **State machine:** `title`, `playing`, `levelup`, `reward`, `gameover`, etc.
- **Progression:** waves, floors, levels, upgrades, score, unlocks, or prestige.
- **Enemy variety:** at least fast, tanky, ranged/status, elite, and boss roles
  when scope allows.
- **Player decisions:** skill cards, item placement, route choice, buy/sell,
  merge, or upgrade selection.
- **Readable feedback:** hit flash, damage numbers, particles, HP bars, phase
  text, logs.
- **Failure and restart:** clear game-over state and a fast restart path.

## Procedural Generation

Use deterministic or bounded randomness:

```js
function waveConfig(wave) {
  const count = Math.min(5 + wave * 2, 35);
  const types = ['slime', 'bat'];
  if (wave >= 3) types.push('skeleton');
  if (wave >= 5) types.push('elite');
  return {
    count,
    types,
    boss: wave % 5 === 0,
    hpMul: 1 + (wave - 1) * 0.2,
  };
}
```

For dungeons:

- Generate rooms first.
- Connect rooms with corridors.
- Place start and exit far apart.
- Add enemies and loot after topology is valid.
- Use fog of war to control information.

## Upgrade Design

Good upgrades change behavior, not only numbers:

- Projectile bounce
- Area expansion
- Status effects
- Summoned helper
- Crit build
- Lifesteal or shield conversion
- Backpack adjacency modifier
- Economy scaling

Use three-card choices for clarity:

```js
const choices = upgradePool
  .filter(upgrade => canOffer(upgrade, player))
  .sort(() => Math.random() - 0.5)
  .slice(0, 3);
```

## Combat Readability

- Draw attacks from source to target.
- Keep enemy HP visible for elites and bosses.
- Use color-coded effects: frost, burn, poison, shock, crit.
- Keep particles short-lived.
- Avoid effects that hide silhouettes.
- Log important phase transitions.

## Mobile Requirements

- Touch controls must be large and stable.
- Turn-based games should support taps/swipes.
- Action games need predictable joystick or button zones.
- Inventory/backpack games need drag threshold, pointer cancel handling, and no
  accidental page scrolling inside the board.

## Acceptance Checklist

- The first 5 minutes are playable.
- There is an infinite or clearly bounded progression path.
- A boss or milestone changes pacing.
- The player makes at least one meaningful decision between runs/waves/floors.
- Restart works without reloading.
- The game is readable at desktop and mobile sizes.
- All major state transitions have UI feedback.

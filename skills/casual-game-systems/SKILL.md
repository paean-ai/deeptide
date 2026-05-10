# Casual Game Systems

Use this skill when creating casual game samples such as idle games, merge
games, backpack games, tower defense, match-3, brick breaker, fishing idle, or
shop management games.

## Goal

Build a complete, satisfying loop with simple controls, visible progression,
clear rewards, and enough systemic depth to stay interesting.

## Loop Patterns

### Idle / Incremental

```text
click/collect -> buy producer -> automate income -> unlock multiplier -> prestige
```

Include:

- exponential costs,
- offline or passive earnings,
- milestone unlocks,
- clear income rate,
- prestige or late-game multiplier if the game is infinite.

### Merge / Collection

```text
buy item -> place on grid -> merge matching levels -> fulfill orders -> unlock rarities
```

Include:

- grid constraints,
- merge streak or mutation chance,
- orders that direct player goals,
- passive income or timed boosts.

### Backpack / Auto Battler

```text
shop -> spatial arrangement -> adjacency bonuses -> auto combat -> reward choice
```

Include:

- polyomino item footprints,
- rotation and collision checks,
- merge or forge recipes,
- adjacency modifiers,
- autonomous combat visualization.

### Tower Defense

```text
build -> start wave -> enemies traverse path -> towers auto target -> upgrade/sell
```

Include:

- fixed path,
- build pads,
- tower roles,
- wave plan,
- rewards and lives,
- targeting rules.

### Match-3

```text
swap -> match -> clear -> cascade -> refill -> special gem -> level target
```

Include:

- adjacent swap validation,
- cascades,
- no-move reshuffle,
- special pieces,
- target progress.

### Arcade Action

```text
control -> hit target -> score/combo -> powerup -> stage escalation
```

Include:

- tight input,
- particles and screen shake,
- combo scoring,
- powerups,
- touch/mouse support.

## System Design Rules

- One primary action should be obvious within 5 seconds.
- Every reward should unlock a new decision or accelerate the next loop.
- Costs should scale faster than income, then upgrades should bend the curve.
- UI should always show current resource, next cost, and current objective.
- Randomness should feel generous but bounded.
- Avoid dead states: if the player cannot act, offer collect, sell, reroll,
  restart, or wait with visible progress.

## Data-Driven Definitions

Prefer catalogs over hardcoded branches:

```js
const TOWERS = {
  arrow: { cost: 55, range: 126, damage: 18, cooldown: 34 },
  cannon: { cost: 85, range: 108, damage: 38, cooldown: 64, splash: 46 },
  frost: { cost: 75, range: 118, damage: 9, cooldown: 46, slow: 0.46 },
};
```

For items:

```js
const ITEM_DEFS = {
  gear: {
    family: 'modifier',
    shape: [[1]],
    cost: 12,
    damageMul: 1.18,
    desc: 'Adjacent weapons deal more damage.',
  },
};
```

## Feedback Patterns

- Floating numbers for income, damage, and rewards.
- Button disabled states with cost text.
- Short combat or event log.
- Progress bars for HP, XP, orders, waves, or level targets.
- Particle bursts for merge, purchase, kill, clear, and upgrade.

## Balancing Heuristics

- First purchase should happen within 10-20 seconds.
- First upgrade should be reachable before boredom.
- Early waves should validate the loop, not punish.
- Boss or milestone every 5-10 loops.
- Give economy items a delayed payoff and combat items immediate payoff.
- Merge games should preserve space pressure but avoid total gridlock.

## Completion Checklist

- The loop can run indefinitely or has a clear win/finish state.
- The player can recover from poor choices.
- Major actions work with mouse and touch.
- All dynamic text fits in cards/buttons on mobile.
- A reset/restart path exists.
- The README explains the loop and how to run the sample.

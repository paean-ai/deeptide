// Upgrade definitions and logic

const UPGRADE_POOL = [
  {
    id: 'vitality',
    name: 'Vitality',
    icon: '❤',
    desc: '+25 Max HP',
    maxStacks: Infinity,
    apply(state) {
      state.player.maxHp += 25;
      state.player.hp += 25;
    },
    getDesc(stack) { return `+${25 * stack} Max HP`; }
  },
  {
    id: 'power',
    name: 'Power',
    icon: '⚔',
    desc: '+5 Attack',
    maxStacks: Infinity,
    apply(state) { state.player.atk += 5; },
    getDesc(stack) { return `+${5 * stack} Attack`; }
  },
  {
    id: 'armor',
    name: 'Fortitude',
    icon: '🛡',
    desc: '+3 Defense',
    maxStacks: Infinity,
    apply(state) { state.player.def += 3; },
    getDesc(stack) { return `+${3 * stack} Defense`; }
  },
  {
    id: 'crit',
    name: 'Precision',
    icon: '🎯',
    desc: '+8% Crit Chance',
    maxStacks: 10,
    apply(state) { state.player.critChance = Math.min(0.85, state.player.critChance + 0.08); },
    getDesc(stack) { return `${Math.round(state.player.critChance * 100)}% Crit (2x dmg)`; }
  },
  {
    id: 'lifesteal',
    name: 'Vampirism',
    icon: '🩸',
    desc: '+12% Lifesteal',
    maxStacks: 8,
    apply(state) { state.player.lifesteal = Math.min(0.95, state.player.lifesteal + 0.12); },
    getDesc(stack) { return `${Math.round(state.player.lifesteal * 100)}% Lifesteal`; }
  },
  {
    id: 'thorns',
    name: 'Thorns',
    icon: '🌵',
    desc: 'Reflect 25% damage',
    maxStacks: 5,
    apply(state) { state.player.thorns = Math.min(1.0, state.player.thorns + 0.25); },
    getDesc(stack) { return `${Math.round(state.player.thorns * 100)}% Reflect`; }
  },
  {
    id: 'regen',
    name: 'Regeneration',
    icon: '💚',
    desc: 'Heal 4 HP per 5 turns',
    maxStacks: 10,
    apply(state) { state.player.regen += 4; },
    getDesc(stack) { return `Heal ${state.player.regen} HP / 5 turns`; }
  },
  {
    id: 'swift',
    name: 'Swiftness',
    icon: '💨',
    desc: '+12% Free Move chance',
    maxStacks: 6,
    apply(state) { state.player.swiftChance = Math.min(0.7, state.player.swiftChance + 0.12); },
    getDesc(stack) { return `${Math.round(state.player.swiftChance * 100)}% Free Move`; }
  },
  {
    id: 'doublestrike',
    name: 'Double Strike',
    icon: '⚡',
    desc: '+10% Double Attack',
    maxStacks: 8,
    apply(state) { state.player.doubleStrikeChance = Math.min(0.8, state.player.doubleStrikeChance + 0.10); },
    getDesc(stack) { return `${Math.round(state.player.doubleStrikeChance * 100)}% Double Strike`; }
  },
  {
    id: 'dodge',
    name: 'Evasion',
    icon: '💫',
    desc: '+8% Dodge Chance',
    maxStacks: 8,
    apply(state) { state.player.dodgeChance = Math.min(0.6, state.player.dodgeChance + 0.08); },
    getDesc(stack) { return `${Math.round(state.player.dodgeChance * 100)}% Dodge`; }
  },
  {
    id: 'berserk',
    name: 'Berserker',
    icon: '🔥',
    desc: '+40% ATK when HP < 30%',
    maxStacks: 1,
    apply(state) { state.player.berserk = true; },
    getDesc() { return 'Massive ATK at low HP'; }
  },
  {
    id: 'shield',
    name: 'Void Shield',
    icon: '🔮',
    desc: '+30 Shield per floor',
    maxStacks: 5,
    apply(state) { state.player.shield += 30; },
    getDesc(stack) { return `${state.player.shield} Shield / floor`; }
  },
  {
    id: 'scout',
    name: 'Scout',
    icon: '👁',
    desc: '+2 Vision Range',
    maxStacks: 5,
    apply(state) { state.player.visionBonus = (state.player.visionBonus || 0) + 2; },
    getDesc(stack) { return `Vision: ${8 + (state.player.visionBonus || 0)} tiles`; }
  },
  {
    id: 'leech',
    name: 'Soul Leech',
    icon: '💀',
    desc: '+2 ATK per kill (this floor)',
    maxStacks: 5,
    apply(state) { state.player.bonusAtkPerKill = (state.player.bonusAtkPerKill || 0) + 2; },
    getDesc(stack) { return `+${(state.player.bonusAtkPerKill || 0)} ATK/kill`; }
  },
  {
    id: 'assassin',
    name: 'Assassin',
    icon: '🗡',
    desc: 'First attack each floor is a guaranteed crit',
    maxStacks: 1,
    apply(state) { state.player.assassin = true; },
    getDesc() { return 'Guaranteed 1st crit/floor'; }
  },
  {
    id: 'phoenix',
    name: 'Phoenix',
    icon: '🐦',
    desc: 'Revive once per floor at 50% HP',
    maxStacks: 3,
    apply(state) { state.player.phoenixCharges = (state.player.phoenixCharges || 0) + 1; },
    getDesc(stack) { return `${state.player.phoenixCharges || 0} revive(s) / floor`; }
  },
  {
    id: 'cleave',
    name: 'Cleave',
    icon: '🌀',
    desc: 'Attacks hit adjacent enemies',
    maxStacks: 3,
    apply(state) { state.player.cleaveRange = (state.player.cleaveRange || 0) + 1; },
    getDesc(stack) { return `Cleave radius: ${(state.player.cleaveRange || 0)}`; }
  },
  {
    id: 'alchemist',
    name: 'Alchemist',
    icon: '🧪',
    desc: 'Potions heal +60% more',
    maxStacks: 4,
    apply(state) { state.player.potionBonus = (state.player.potionBonus || 0) + 0.6; },
    getDesc(stack) { return `Potions: +${Math.round((state.player.potionBonus || 0) * 100)}%`; }
  },
  {
    id: 'poison',
    name: 'Venom',
    icon: '☠',
    desc: 'Attacks poison for 3 turns',
    maxStacks: 5,
    apply(state) { state.player.poisonDmg = (state.player.poisonDmg || 0) + 5; },
    getDesc(stack) { return `${(state.player.poisonDmg || 0)} poison dmg/turn`; }
  },
  {
    id: 'fortune',
    name: 'Fortune',
    icon: '🍀',
    desc: 'Enemies have 10% miss chance',
    maxStacks: 5,
    apply(state) { state.player.enemyMissChance = (state.player.enemyMissChance || 0) + 0.10; },
    getDesc(stack) { return `${Math.round((state.player.enemyMissChance || 0) * 100)}% enemy miss`; }
  },
  {
    id: 'glasscanon',
    name: 'Glass Cannon',
    icon: '💎',
    desc: '+12 ATK, -8 Max HP',
    maxStacks: 6,
    apply(state) {
      state.player.atk += 12;
      state.player.maxHp = Math.max(20, state.player.maxHp - 8);
      state.player.hp = Math.min(state.player.hp, state.player.maxHp);
    },
    getDesc() { return '+12 ATK, -8 Max HP'; }
  },
  {
    id: 'juggernaut',
    name: 'Juggernaut',
    icon: '🦾',
    desc: '+15 Max HP, -2 ATK',
    maxStacks: 6,
    apply(state) {
      state.player.maxHp += 15;
      state.player.hp += 15;
      state.player.atk = Math.max(1, state.player.atk - 2);
    },
    getDesc() { return '+15 HP, -2 ATK'; }
  },
];

function getRandomUpgrades(count, stateUpgrades) {
  // stateUpgrades is { upgradeId: stackCount }
  const available = UPGRADE_POOL.filter(u => {
    const current = stateUpgrades[u.id] || 0;
    return current < u.maxStacks;
  });
  const shuffled = [...available].sort(() => Math.random() - 0.5);
  const chosen = [];
  const seen = new Set();
  for (const u of shuffled) {
    if (!seen.has(u.id)) {
      chosen.push(u);
      seen.add(u.id);
    }
    if (chosen.length >= count) break;
  }
  return chosen;
}

function applyUpgrade(state, upgrade) {
  upgrade.apply(state);
  state.upgrades[upgrade.id] = (state.upgrades[upgrade.id] || 0) + 1;
}

function getUpgradeCardHTML(upgrade, index) {
  const color = upgradeColor(upgrade.id);
  return `
    <div class="upgrade-card" onclick="selectUpgrade(${index})">
      <canvas class="upgrade-icon pixel-upgrade-glyph" width="56" height="56" data-upgrade="${upgrade.id}" data-color="${color}"></canvas>
      <div class="upgrade-name">${typeof upgradeName === 'function' ? upgradeName(upgrade) : upgrade.name}</div>
      <div class="upgrade-desc">${typeof upgradeDesc === 'function' ? upgradeDesc(upgrade) : upgrade.desc}</div>
    </div>
  `;
}

function upgradeColor(id) {
  const colors = {
    vitality: '#43d17a',
    power: '#f2c14e',
    armor: '#2f80ed',
    crit: '#f3f7ff',
    lifesteal: '#e05243',
    thorns: '#43d17a',
    regen: '#b5f47a',
    swift: '#ff8a3d',
    doublestrike: '#f2c14e',
    dodge: '#a9e8ff',
    berserk: '#ff8a3d',
    shield: '#a9e8ff',
    scout: '#b66cff',
    leech: '#e05243',
    assassin: '#f3f7ff',
    phoenix: '#f2c14e',
    cleave: '#b66cff',
    alchemist: '#43d17a',
    poison: '#43d17a',
    fortune: '#f2c14e',
    glasscanon: '#f3f7ff',
    juggernaut: '#8b93a1',
  };
  return colors[id] || '#f2c14e';
}

function renderUpgradeGlyphs() {
  document.querySelectorAll('.pixel-upgrade-glyph').forEach(canvas => {
    const ctx = canvas.getContext('2d');
    ctx.imageSmoothingEnabled = false;
    vdDrawGlyph(ctx, canvas.dataset.upgrade, 0, 0, canvas.width, canvas.dataset.color);
  });
}

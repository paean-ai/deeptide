const SAVE_KEY = 'starforge-idle-save-v1';

const ITEMS = {
  upgrades: [
    {
      id: 'pick',
      name: { en: 'Starsteel Pick', zh: '星钢镐' },
      color: '#67e7ff',
      resource: 'dust',
      base: 12,
      growth: 1.18,
      desc: { en: lv => `Click stardust +${1 + lv}`, zh: lv => `点击星尘 +${1 + lv}` },
      effect: s => { s.clickPower += levelOf(s, 'pick'); },
    },
    {
      id: 'glove',
      name: { en: 'Gravity Glove', zh: '引力手套' },
      color: '#51d889',
      resource: 'dust',
      base: 85,
      growth: 1.22,
      desc: { en: lv => `Click multiplier +${lv * 8}%`, zh: lv => `点击倍率 +${lv * 8}%` },
      effect: s => { s.clickMul *= 1 + levelOf(s, 'glove') * 0.08; },
    },
    {
      id: 'smelter',
      name: { en: 'Micro Smelter', zh: '微型熔炉' },
      color: '#f4c656',
      resource: 'dust',
      base: 160,
      growth: 1.2,
      desc: { en: lv => `Alloy conversion +${lv * 12}%`, zh: lv => `合金转化效率 +${lv * 12}%` },
      effect: s => { s.alloyMul *= 1 + levelOf(s, 'smelter') * 0.12; },
    },
    {
      id: 'lens',
      name: { en: 'Star Map Lens', zh: '星图透镜' },
      color: '#aa7dff',
      resource: 'alloy',
      base: 50,
      growth: 1.24,
      desc: { en: lv => `All production +${lv * 5}%`, zh: lv => `所有产出 +${lv * 5}%` },
      effect: s => { s.globalMul *= 1 + levelOf(s, 'lens') * 0.05; },
    },
  ],
  machines: [
    {
      id: 'drone',
      name: { en: 'Mining Drone', zh: '采集无人机' },
      color: '#67e7ff',
      resource: 'dust',
      base: 30,
      growth: 1.16,
      desc: { en: lv => `Auto stardust +${format(0.8 * (lv + 1))}/s`, zh: lv => `自动星尘 +${format(0.8 * (lv + 1))}/s` },
      effect: s => { s.dustPerSec += levelOf(s, 'drone') * 0.8; },
    },
    {
      id: 'rig',
      name: { en: 'Orbital Rig', zh: '轨道钻机' },
      color: '#e75b58',
      resource: 'dust',
      base: 260,
      growth: 1.19,
      desc: { en: lv => `Auto stardust +${format(5 * (lv + 1))}/s`, zh: lv => `自动星尘 +${format(5 * (lv + 1))}/s` },
      effect: s => { s.dustPerSec += levelOf(s, 'rig') * 5; },
    },
    {
      id: 'foundry',
      name: { en: 'Auto Foundry', zh: '自动铸造线' },
      color: '#f4c656',
      resource: 'dust',
      base: 900,
      growth: 1.21,
      desc: { en: lv => `Auto alloy +${format(0.22 * (lv + 1))}/s`, zh: lv => `自动合金 +${format(0.22 * (lv + 1))}/s` },
      effect: s => { s.alloyPerSec += levelOf(s, 'foundry') * 0.22; },
    },
    {
      id: 'reactor',
      name: { en: 'Core Reactor', zh: '核心反应堆' },
      color: '#aa7dff',
      resource: 'alloy',
      base: 650,
      growth: 1.25,
      desc: { en: lv => `Auto core +${format(0.035 * (lv + 1))}/s`, zh: lv => `自动核心 +${format(0.035 * (lv + 1))}/s` },
      effect: s => { s.corePerSec += levelOf(s, 'reactor') * 0.035; },
    },
  ],
  relics: [
    {
      id: 'memory',
      name: { en: 'Ancient Cache', zh: '远古缓存' },
      color: '#67e7ff',
      resource: 'core',
      base: 2,
      growth: 1.28,
      desc: { en: lv => `Offline earnings +${lv * 10}%`, zh: lv => `离线收益 +${lv * 10}%` },
      effect: s => { s.offlineMul *= 1 + levelOf(s, 'memory') * 0.1; },
    },
    {
      id: 'crown',
      name: { en: 'Stardust Crown', zh: '星尘王冠' },
      color: '#f4c656',
      resource: 'core',
      base: 6,
      growth: 1.31,
      desc: { en: lv => `Prestige multiplier +${lv * 8}%`, zh: lv => `声望倍率 +${lv * 8}%` },
      effect: s => { s.prestigeMul *= 1 + levelOf(s, 'crown') * 0.08; },
    },
    {
      id: 'engine',
      name: { en: 'Perpetual Engine', zh: '永动引擎' },
      color: '#51d889',
      resource: 'core',
      base: 12,
      growth: 1.33,
      desc: { en: lv => `All auto production +${lv * 11}%`, zh: lv => `所有自动产出 +${lv * 11}%` },
      effect: s => { s.autoMul *= 1 + levelOf(s, 'engine') * 0.11; },
    },
  ],
};

function levelOf(state, id) {
  return state.levels[id] || 0;
}

function costFor(item, level) {
  return Math.floor(item.base * Math.pow(item.growth, level));
}

function allItems() {
  return [...ITEMS.upgrades, ...ITEMS.machines, ...ITEMS.relics];
}

function format(n) {
  if (!Number.isFinite(n)) return 'inf';
  if (n < 1000) return n % 1 === 0 ? String(Math.floor(n)) : n.toFixed(n < 10 ? 2 : 1);
  const units = ['', 'K', 'M', 'B', 'T', 'Qa', 'Qi', 'Sx', 'Sp', 'Oc', 'No', 'Dc'];
  let u = 0;
  while (n >= 1000 && u < units.length - 1) {
    n /= 1000;
    u++;
  }
  return `${n.toFixed(n < 10 ? 2 : 1)}${units[u]}`;
}

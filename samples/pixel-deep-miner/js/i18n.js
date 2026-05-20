// Pixel Deep Miner - localization (English / 中文)
const LANG_KEY = 'pixel-deep-miner-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'DEEP MINER',
    subtitle: 'Dig for ore, dodge the lava, surface to bank it.',
    play: 'START DIGGING', resume: 'CONTINUE', howto: 'Arrows / WASD or swipe. Dig down — fly up to sell.',
    depth: 'Depth', cash: 'Cash', fuel: 'Fuel', hull: 'Hull', cargo: 'Cargo',
    shop: 'SHOP', sellAll: 'Sell All', close: 'Back to Surface', leave: 'Resume',
    paused: 'PAUSED', restart: 'New Miner', menu: 'Menu',
    gameOver: 'RIG LOST', strandedFuel: 'Out of fuel — stranded in the dark.',
    strandedHull: 'Hull breached — the rig is scrap.',
    reachedDepth: d => `Deepest dig: ${d} m.`,
    bestDepth: d => `Record depth: ${d} m`,
    again: 'NEW MINER', sold: (n, c) => `Sold ${n} ore for ◆${c}.`,
    nothingToSell: 'Cargo hold is empty.', cargoFull: 'Cargo hold is full!',
    bought: n => `Upgraded ${n}.`, refuel: 'Refuelled and repaired at the surface.',
    maxed: 'MAX', owned: 'Owned', buy: 'Buy', need: 'Need ◆',
    oreNames: { copper: 'Copper', iron: 'Iron', silver: 'Silver', cobalt: 'Cobalt', gold: 'Gold', gem: 'Gem', mythril: 'Mythril' },
    upNames: { drill: 'Drill', fuel: 'Fuel Tank', cargo: 'Cargo Hold', hull: 'Hull', thruster: 'Thruster' },
    upDesc: {
      drill: 'Drill harder rock, faster.', fuel: 'Carry more fuel underground.',
      cargo: 'Haul more ore per trip.', hull: 'More HP and lava resistance.',
      thruster: 'Burn less fuel flying upward.',
    },
    tip: 'Surface refuels & repairs free. Watch your fuel before the climb.',
  },
  zh: {
    title: '深井矿工',
    subtitle: '下挖采矿，躲避岩浆，回到地面变现。',
    play: '开始挖掘', resume: '继续', howto: '方向键 / WASD 或滑动。向下挖矿，向上飞行卖矿。',
    depth: '深度', cash: '现金', fuel: '燃料', hull: '船体', cargo: '货舱',
    shop: '商店', sellAll: '全部卖出', close: '返回地面', leave: '继续',
    paused: '已暂停', restart: '新矿机', menu: '菜单',
    gameOver: '矿机损毁', strandedFuel: '燃料耗尽 —— 被困在黑暗中。',
    strandedHull: '船体破裂 —— 矿机报废了。',
    reachedDepth: d => `最深挖掘：${d} 米。`,
    bestDepth: d => `深度记录：${d} 米`,
    again: '新矿机', sold: (n, c) => `卖出 ${n} 块矿石，获得 ◆${c}。`,
    nothingToSell: '货舱是空的。', cargoFull: '货舱已满！',
    bought: n => `升级了${n}。`, refuel: '在地面免费补充燃料并修复。',
    maxed: '满级', owned: '已拥有', buy: '购买', need: '需要 ◆',
    oreNames: { copper: '铜矿', iron: '铁矿', silver: '银矿', cobalt: '钴矿', gold: '金矿', gem: '宝石', mythril: '秘银' },
    upNames: { drill: '钻头', fuel: '燃料箱', cargo: '货舱', hull: '船体', thruster: '推进器' },
    upDesc: {
      drill: '更快钻穿更硬的岩石。', fuel: '在地下携带更多燃料。',
      cargo: '每趟运载更多矿石。', hull: '更高生命值与岩浆抗性。',
      thruster: '向上飞行时消耗更少燃料。',
    },
    tip: '地面免费补给与维修。攀升前留意燃料。',
  },
};

function t(key, ...args) {
  const v = TEXT[currentLang][key] ?? TEXT.en[key] ?? key;
  return typeof v === 'function' ? v(...args) : v;
}
function tOre(id) { return TEXT[currentLang].oreNames[id] || id; }
function tUpName(id) { return TEXT[currentLang].upNames[id] || id; }
function tUpDesc(id) { return TEXT[currentLang].upDesc[id] || ''; }

function applyStaticText() {
  document.documentElement.lang = currentLang === 'zh' ? 'zh-CN' : 'en';
  document.querySelectorAll('[data-i18n]').forEach(el => { el.textContent = t(el.dataset.i18n); });
}
function setupLanguageToggle(onChange) {
  const btn = document.getElementById('lang-toggle');
  if (btn) {
    btn.onclick = () => {
      currentLang = currentLang === 'en' ? 'zh' : 'en';
      localStorage.setItem(LANG_KEY, currentLang);
      btn.textContent = currentLang === 'en' ? '中文' : 'EN';
      applyStaticText();
      if (onChange) onChange();
    };
    btn.textContent = currentLang === 'en' ? '中文' : 'EN';
  }
  applyStaticText();
}

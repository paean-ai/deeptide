// Pixel Town Tycoon - localization
const LANG_KEY = 'pixel-town-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'PIXEL TOWN TYCOON',
    subtitle: 'Place, supply, and grow a thriving town.',
    play: 'PLAY',
    howto: 'Tap a building, then tap a tile to place it. Tap a building to upgrade it.',
    coins: 'Coins', population: 'Pop', rank: 'Rank',
    build: 'Build', upgrade: 'Upgrade', demolish: 'Demolish', sell: 'Sell',
    quests: 'Quests', close: 'Close',
    workers: 'Workers', staffed: 'Staffed', understaffed: 'Understaffed',
    level: 'Lv', maxLevel: 'MAX',
    produces: 'Produces', consumes: 'Needs', sells: 'Sells goods',
    storage: 'Storage', housing: 'Housing',
    adjBonus: 'Adjacency bonus active', adjHint: pair => `Place next to ${pair} for +15%`,
    locked: 'Unlocks at higher rank',
    cantAfford: 'Not enough coins',
    occupied: 'Tile occupied', blocked: 'Cannot build here',
    rankUp: r => `Town is now a ${r}!`,
    questDone: 'Quest complete!',
    welcomeBack: s => `Welcome back! Earned ${s} coins while away.`,
    offline: 'Offline earnings',
    perSec: '/s', capReached: 'Storage full',
    allQuests: 'All quests complete — your town thrives!',
    reward: 'Reward',
    bNames: {
      cottage: 'Cottage', farm: 'Farm', well: 'Well', lumber: 'Lumber Camp',
      market: 'Market', mine: 'Mine', mill: 'Mill', sawmill: 'Sawmill',
      bakery: 'Bakery', smithy: 'Smithy', warehouse: 'Warehouse',
    },
    rNames: {
      wheat: 'Wheat', water: 'Water', wood: 'Wood', ore: 'Ore',
      flour: 'Flour', plank: 'Plank', bread: 'Bread', tools: 'Tools',
    },
  },
  zh: {
    title: '像素小镇大亨',
    subtitle: '布局、供应链、打造繁荣小镇。',
    play: '开始游戏',
    howto: '点击建筑，再点击地块进行放置。点击已有建筑可升级。',
    coins: '金币', population: '人口', rank: '等级',
    build: '建造', upgrade: '升级', demolish: '拆除', sell: '出售',
    quests: '任务', close: '关闭',
    workers: '工人', staffed: '满员', understaffed: '人手不足',
    level: '等级', maxLevel: '满级',
    produces: '产出', consumes: '消耗', sells: '出售商品',
    storage: '仓储', housing: '住房',
    adjBonus: '相邻加成生效', adjHint: pair => `相邻 ${pair} 可获 +15%`,
    locked: '更高等级解锁',
    cantAfford: '金币不足',
    occupied: '地块已占用', blocked: '此处无法建造',
    rankUp: r => `小镇升级为「${r}」！`,
    questDone: '任务完成！',
    welcomeBack: s => `欢迎回来！离线期间赚取了 ${s} 金币。`,
    offline: '离线收益',
    perSec: '/秒', capReached: '仓库已满',
    allQuests: '所有任务完成——你的小镇欣欣向荣！',
    reward: '奖励',
    bNames: {
      cottage: '小屋', farm: '农场', well: '水井', lumber: '伐木场',
      market: '集市', mine: '矿场', mill: '磨坊', sawmill: '锯木厂',
      bakery: '面包房', smithy: '铁匠铺', warehouse: '仓库',
    },
    rNames: {
      wheat: '小麦', water: '水', wood: '木材', ore: '矿石',
      flour: '面粉', plank: '木板', bread: '面包', tools: '工具',
    },
  },
};

function t(key, ...args) {
  const v = TEXT[currentLang][key] ?? TEXT.en[key] ?? key;
  return typeof v === 'function' ? v(...args) : v;
}
function bName(id) { return TEXT[currentLang].bNames[id] || id; }
function rName(id) { return TEXT[currentLang].rNames[id] || id; }
function rankName(idx) {
  const r = RANKS[Math.min(idx, RANKS.length - 1)];
  return currentLang === 'zh' ? r.name[1] : r.name[0];
}
function questText(q) { return currentLang === 'zh' ? q.text[1] : q.text[0]; }

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

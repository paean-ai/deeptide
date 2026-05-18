// Pixel Match Quest - localization
const LANG_KEY = 'pixel-match-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'PIXEL MATCH QUEST',
    subtitle: 'Swap, match, and clear every objective.',
    play: 'PLAY',
    howto: 'Swap two adjacent gems to line up 3+. Match 4 or 5 for powerful gems.',
    selectLevel: 'SELECT LEVEL', back: 'Back', locked: 'LOCKED',
    level: 'Level', moves: 'Moves', score: 'Score', coins: 'Coins',
    objective: 'Goal',
    objScore: n => `Reach ${n} points`,
    objColor: (n, c) => `Clear ${n} ${c} gems`,
    objIce: 'Clear all ice',
    objCrate: 'Break all crates',
    objDrop: n => `Drop ${n} fruits to the bottom`,
    levelClear: 'LEVEL CLEAR', failed: 'OUT OF MOVES',
    tryAgain: 'Retry', nextLevel: 'Next', menu: 'Map',
    starsEarned: 'Stars', coinsEarned: c => `+${c} coins`,
    boosters: 'Boosters',
    buyBooster: cost => `Buy ◆${cost}`,
    notEnough: 'Not enough coins',
    useHammer: 'Tap a gem to smash it', shuffled: 'Board shuffled',
    movesAdded: '+5 moves', noMoves: 'No moves — shuffling',
    colorNames: ['Red', 'Blue', 'Green', 'Yellow', 'Purple', 'Orange'],
    paused: 'PAUSED', resume: 'Resume', quit: 'Quit',
    comboWord: 'Combo!', sweetWord: 'Sweet!', greatWord: 'Great!',
  },
  zh: {
    title: '像素消除冒险',
    subtitle: '交换、消除，完成每个目标。',
    play: '开始游戏',
    howto: '交换相邻的两颗宝石连成 3 个以上。消除 4 或 5 个可生成强力宝石。',
    selectLevel: '选择关卡', back: '返回', locked: '未解锁',
    level: '关卡', moves: '步数', score: '分数', coins: '金币',
    objective: '目标',
    objScore: n => `达到 ${n} 分`,
    objColor: (n, c) => `消除 ${n} 个${c}宝石`,
    objIce: '清除所有冰块',
    objCrate: '打破所有木箱',
    objDrop: n => `让 ${n} 个果实落到底部`,
    levelClear: '过关！', failed: '步数用尽',
    tryAgain: '重试', nextLevel: '下一关', menu: '地图',
    starsEarned: '星星', coinsEarned: c => `+${c} 金币`,
    boosters: '道具',
    buyBooster: cost => `购买 ◆${cost}`,
    notEnough: '金币不足',
    useHammer: '点击一颗宝石将其砸碎', shuffled: '棋盘已重排',
    movesAdded: '+5 步', noMoves: '无可消除 — 重排中',
    colorNames: ['红色', '蓝色', '绿色', '黄色', '紫色', '橙色'],
    paused: '已暂停', resume: '继续', quit: '退出',
    comboWord: '连击！', sweetWord: '漂亮！', greatWord: '不错！',
  },
};

function t(key, ...args) {
  const v = TEXT[currentLang][key] ?? TEXT.en[key] ?? key;
  return typeof v === 'function' ? v(...args) : v;
}
function colorName(i) { return TEXT[currentLang].colorNames[i] || ''; }

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

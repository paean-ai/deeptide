// Pixel Mind Match - localization (English / 中文)
const LANG_KEY = 'pixel-mind-match-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'MIND MATCH',
    subtitle: 'Flip the tiles. Remember the creatures. Match every pair.',
    play: 'PLAY', levelSelect: 'LEVELS',
    howto: 'Tap two tiles to flip them. Match all pairs in as few moves as you can.',
    level: 'Level', moves: 'Moves', combo: 'Combo', best: 'Best',
    restart: 'Restart', menu: 'Menu', next: 'NEXT LEVEL', memorise: 'MEMORISE!',
    cleared: 'ALL MATCHED', clearedMsg: m => `Cleared in ${m} moves.`,
    bestMoves: m => `Best: ${m} moves`, newBest: 'NEW BEST!',
    allDone: 'MIND MASTER', allDoneMsg: 'Every board cleared. A flawless memory!',
    locked: 'Locked',
  },
  zh: {
    title: '记忆配对',
    subtitle: '翻开方块,记住小动物,配对所有卡牌。',
    play: '开始', levelSelect: '选关',
    howto: '点击两张牌翻开,用尽量少的步数配对所有卡牌。',
    level: '关卡', moves: '步数', combo: '连击', best: '最佳',
    restart: '重来', menu: '菜单', next: '下一关', memorise: '记住它们！',
    cleared: '全部配对', clearedMsg: m => `用 ${m} 步通关。`,
    bestMoves: m => `最佳：${m} 步`, newBest: '新纪录！',
    allDone: '记忆大师', allDoneMsg: '所有牌局全部通关,记忆完美无瑕！',
    locked: '未解锁',
  },
};

function t(key, ...args) {
  const v = TEXT[currentLang][key] ?? TEXT.en[key] ?? key;
  return typeof v === 'function' ? v(...args) : v;
}
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

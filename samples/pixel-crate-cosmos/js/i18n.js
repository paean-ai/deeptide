// Pixel Crate Cosmos - localization (English / 中文)
const LANG_KEY = 'pixel-crate-cosmos-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'CRATE COSMOS',
    subtitle: 'Push every power-core onto its socket.',
    play: 'PLAY', levelSelect: 'LEVELS', howto: 'Arrows / WASD or swipe to move. Slide on ice. Undo freely.',
    level: 'Level', moves: 'Moves', best: 'Best',
    undo: 'Undo', restart: 'Restart', menu: 'Menu', next: 'NEXT LEVEL',
    cleared: 'STATION ONLINE', clearedMsg: m => `Solved in ${m} moves.`,
    bestMoves: m => `Best: ${m} moves`, newBest: 'NEW BEST!',
    allDone: 'COSMOS RESTORED', allDoneMsg: 'Every station is back online. Stellar work!',
    locked: 'Locked', tip: 'Tip: a core in a corner can never move again.',
  },
  zh: {
    title: '星舱推箱',
    subtitle: '把每个能量核推上对应的接口。',
    play: '开始', levelSelect: '选关', howto: '方向键 / WASD 或滑动移动。冰面会滑行。可随意撤销。',
    level: '关卡', moves: '步数', best: '最佳',
    undo: '撤销', restart: '重来', menu: '菜单', next: '下一关',
    cleared: '空间站已上线', clearedMsg: m => `用 ${m} 步通关。`,
    bestMoves: m => `最佳：${m} 步`, newBest: '新纪录！',
    allDone: '星海已修复', allDoneMsg: '所有空间站都已重新上线，干得漂亮！',
    locked: '未解锁', tip: '提示：被推进角落的核就再也推不动了。',
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

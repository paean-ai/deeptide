// Pixel Brick Knight - localization (English / 中文)
const LANG_KEY = 'pixel-brick-knight-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'BRICK KNIGHT',
    subtitle: 'Smash the dungeon floors. Grow stronger each one.',
    howto: 'Drag to steer the paddle. Clear every brick on a floor, then pick a power to carry deeper.',
    play: 'DESCEND', again: 'DESCEND AGAIN', menu: 'MENU',
    floorClear: 'FLOOR CLEARED', pickPower: 'Choose a power',
    gameOver: 'KNIGHT FALLEN',
    finalLine: (fl, gd) => `Reached floor ${fl} · ${gd} gold`,
    bestLine: fl => `Best: floor ${fl}`,
    newBest: 'NEW BEST!',
  },
  zh: {
    title: '砖块骑士',
    subtitle: '砸穿地牢楼层,每层都变得更强。',
    howto: '拖动控制挡板。清掉一层的所有砖块,然后挑选一项强化继续深入。',
    play: '下潜', again: '再次下潜', menu: '菜单',
    floorClear: '楼层清空', pickPower: '选择一项强化',
    gameOver: '骑士倒下',
    finalLine: (fl, gd) => `抵达第 ${fl} 层 · ${gd} 金币`,
    bestLine: fl => `最佳:第 ${fl} 层`,
    newBest: '新纪录！',
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

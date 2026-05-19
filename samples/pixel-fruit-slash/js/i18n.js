// Pixel Fruit Slash - localization (English / 中文)
const LANG_KEY = 'pixel-fruit-slash-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'FRUIT SLASH',
    subtitle: 'Swipe to slice. Chain combos. Never touch a bomb.',
    howto: 'Drag across the fruit to slice it. Slice several in one swipe for a combo bonus.',
    keys: 'Let three fruit drop and the run ends — slicing a bomb ends it instantly.',
    play: 'SLICE', again: 'SLICE AGAIN', menu: 'MENU',
    score: 'SCORE', best: 'BEST',
    gameOver: 'RUN OVER', boom: 'BOMB!',
    comboLabel: n => n + ' COMBO!',
    finalScore: s => `You scored ${s}`,
    newBest: 'NEW BEST!',
  },
  zh: {
    title: '极速切果',
    subtitle: '滑动切水果,连成连击,千万别碰炸弹。',
    howto: '划过水果即可切开。一次滑动切中多个可获得连击奖励。',
    keys: '漏掉三个水果游戏即结束 —— 切到炸弹则立刻结束。',
    play: '开切', again: '再切一局', menu: '菜单',
    score: '分数', best: '最高',
    gameOver: '游戏结束', boom: '炸弹！',
    comboLabel: n => n + ' 连击！',
    finalScore: s => `你的得分 ${s}`,
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

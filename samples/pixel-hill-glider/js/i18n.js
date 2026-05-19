// Pixel Hill Glider - localization (English / 中文)
const LANG_KEY = 'pixel-hill-glider-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'HILL GLIDER',
    subtitle: 'Dive the downhills, soar off the crests.',
    howto: 'Hold anywhere (or Space) to tuck and dive. Release to glide. Catch light orbs before dusk falls.',
    play: 'GLIDE', again: 'GLIDE AGAIN', menu: 'MENU',
    dist: 'DIST', best: 'BEST', fever: 'FEVER',
    gameOver: 'DUSK FELL', orbs: 'Orbs',
    finalDist: d => `You glided ${d} m`,
    newBest: 'NEW BEST!',
  },
  zh: {
    title: '滑翔山丘',
    subtitle: '俯冲下坡,从山脊一跃冲天。',
    howto: '按住屏幕任意处(或空格)收翅俯冲,松手滑翔。在天黑前接住光球。',
    play: '滑翔', again: '再次滑翔', menu: '菜单',
    dist: '距离', best: '最佳', fever: '热度',
    gameOver: '夜幕降临', orbs: '光球',
    finalDist: d => `你滑翔了 ${d} 米`,
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

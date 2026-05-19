// Pixel Tower Stack - localization (English / 中文)
const LANG_KEY = 'pixel-tower-stack-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'TOWER STACK',
    subtitle: 'One tap. Stack it clean. Reach the sky.',
    play: 'STACK', howto: 'Tap, click, or press space to drop the block. Line it up perfectly.',
    height: 'Height', best: 'Best', combo: 'Combo',
    paused: 'PAUSED', resume: 'Resume', restart: 'Restart', menu: 'Menu',
    gameOver: 'TOPPLED', finalHeight: h => `Tower height: ${h}`,
    bestHeight: h => `Best: ${h}`, again: 'STACK AGAIN',
    perfect: 'PERFECT', tapToDrop: 'TAP TO DROP',
  },
  zh: {
    title: '叠叠高塔',
    subtitle: '一键叠放，对齐精准，直冲云霄。',
    play: '叠塔', howto: '点击、单击或按空格放下方块，尽量对齐。',
    height: '高度', best: '最高', combo: '连击',
    paused: '已暂停', resume: '继续', restart: '重新开始', menu: '菜单',
    gameOver: '塔倒了', finalHeight: h => `塔高：${h}`,
    bestHeight: h => `最高：${h}`, again: '再叠一次',
    perfect: '完美', tapToDrop: '点击放下',
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

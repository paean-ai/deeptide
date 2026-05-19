// Pixel Sky Climber - localization (English / 中文)
const LANG_KEY = 'pixel-sky-climber-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'SKY CLIMBER',
    subtitle: 'Bounce higher. Never look down.',
    play: 'CLIMB', howto: 'Tilt with arrows / A·D, or hold the left / right of the screen. You bounce on your own.',
    height: 'Height', score: 'Score', best: 'Best', coins: 'Coins',
    paused: 'PAUSED', resume: 'Resume', restart: 'Restart', menu: 'Menu',
    gameOver: 'YOU FELL', reachedHeight: h => `You climbed ${h} m.`,
    bestHeight: h => `Best: ${h} m`, again: 'CLIMB AGAIN',
    jetpack: 'JETPACK!', spring: 'BOING!',
  },
  zh: {
    title: '云霄攀登者',
    subtitle: '越弹越高，绝不向下看。',
    play: '攀登', howto: '用方向键 / A·D 倾斜，或按住屏幕左 / 右半边。角色会自动弹跳。',
    height: '高度', score: '分数', best: '最高', coins: '金币',
    paused: '已暂停', resume: '继续', restart: '重新开始', menu: '菜单',
    gameOver: '你坠落了', reachedHeight: h => `你攀登了 ${h} 米。`,
    bestHeight: h => `最高：${h} 米`, again: '再次攀登',
    jetpack: '喷气背包！', spring: '弹起！',
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

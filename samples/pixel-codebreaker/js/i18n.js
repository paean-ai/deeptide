// Pixel Codebreaker - localization (English / 中文)
const LANG_KEY = 'pixel-codebreaker-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'CODEBREAKER',
    subtitle: 'Crack the hidden colour code by deduction.',
    howto: 'Tap colours to build a guess, then submit it. Solid pegs mean right colour & spot; hollow pegs mean right colour, wrong spot.',
    play: 'PLAY', levelSelect: 'LEVELS', menu: 'MENU', retry: 'RETRY', next: 'NEXT',
    level: 'LEVEL', tries: 'TRIES',
    win: 'CODE CRACKED!', perfect: 'MASTER MIND!',
    lose: 'OUT OF TRIES',
    winLine: n => `Solved in ${n} guesses`,
    loseLine: 'The code is revealed above — try again.',
    locked: 'LOCKED',
  },
  zh: {
    title: '密码破译',
    subtitle: '靠推理破解隐藏的颜色密码。',
    howto: '点击颜色组成一次猜测,然后提交。实心钉=颜色和位置都对;空心钉=颜色对、位置错。',
    play: '开始', levelSelect: '选关', menu: '菜单', retry: '重试', next: '下一关',
    level: '关卡', tries: '次数',
    win: '密码破解！', perfect: '大师之脑！',
    lose: '次数用尽',
    winLine: n => `${n} 次猜中`,
    loseLine: '密码已在上方揭晓 —— 再试一次。',
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

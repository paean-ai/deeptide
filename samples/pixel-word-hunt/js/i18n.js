// Pixel Word Hunt - localization (English / 中文)
const LANG_KEY = 'pixel-word-hunt-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'WORD HUNT',
    subtitle: 'Drag across the letters to spot every hidden word.',
    howto: 'Words hide in any direction — across, down and diagonally, forwards or backwards. Drag from the first letter to the last.',
    play: 'PLAY', levelSelect: 'PUZZLES', menu: 'MENU', retry: 'RETRY', next: 'NEXT',
    win: 'ALL FOUND!', perfect: 'EAGLE EYE!',
    winLine: (tm, h) => `Time ${tm} · ${h} hints`,
    locked: 'LOCKED',
  },
  zh: {
    title: '找词猎人',
    subtitle: '在字母间拖动,找出所有隐藏单词。',
    howto: '单词藏在任意方向 —— 横、竖、斜,正读或反读。从首字母拖到尾字母。',
    play: '开始', levelSelect: '关卡', menu: '菜单', retry: '重玩', next: '下一关',
    win: '全部找到！', perfect: '火眼金睛！',
    winLine: (tm, h) => `用时 ${tm} · 提示 ${h} 次`,
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

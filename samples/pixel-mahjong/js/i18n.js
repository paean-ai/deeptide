// Pixel Mahjong - localization (English / 中文)
const LANG_KEY = 'pixel-mahjong-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'MAHJONG',
    subtitle: 'Match free tiles. Clear the whole stack.',
    howto: 'Tap two matching tiles that have a free left or right edge and nothing on top. Clear every tile to win.',
    play: 'PLAY', levelSelect: 'LEVELS', menu: 'MENU', retry: 'RETRY', next: 'NEXT',
    level: 'LEVEL', tiles: 'TILES',
    win: 'STACK CLEARED!', perfect: 'FLAWLESS!',
    winLine: (tm, sh) => `Time ${tm} · ${sh} shuffles`,
    locked: 'LOCKED',
    noMoves: 'NO MOVES — SHUFFLE',
  },
  zh: {
    title: '麻将连连',
    subtitle: '消除自由的牌,清空整座牌山。',
    howto: '点击两张相同的牌:它们的左边或右边要空着,且头顶没有牌。清空所有牌即获胜。',
    play: '开始', levelSelect: '选关', menu: '菜单', retry: '重玩', next: '下一关',
    level: '关卡', tiles: '剩余',
    win: '牌山清空！', perfect: '完美通关！',
    winLine: (tm, sh) => `用时 ${tm} · 洗牌 ${sh} 次`,
    locked: '未解锁',
    noMoves: '无可消除 —— 请洗牌',
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

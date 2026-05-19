// Pixel Solitaire - localization (English / 中文)
const LANG_KEY = 'pixel-solitaire-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'SOLITAIRE',
    subtitle: 'Klondike patience, one tap at a time.',
    howto: 'Tap a card to send it to its best spot. Tap the deck to draw. Build the four foundations from Ace to King.',
    play: 'DEAL', again: 'NEW DEAL', menu: 'MENU',
    won: 'YOU WIN!',
    wonLine: (tm, mv) => `Solved in ${tm} · ${mv} moves`,
    statsLine: (n, tm) => n > 0 ? `${n} games won · best ${tm}` : 'No games won yet',
  },
  zh: {
    title: '纸牌接龙',
    subtitle: '克朗代克接龙,一次一点。',
    howto: '点击一张牌,它会自动移到最合适的位置。点击牌堆发牌。把四个基础堆从 A 叠到 K。',
    play: '发牌', again: '再来一局', menu: '菜单',
    won: '你赢了！',
    wonLine: (tm, mv) => `${tm} 完成 · ${mv} 步`,
    statsLine: (n, tm) => n > 0 ? `已胜 ${n} 局 · 最佳 ${tm}` : '还没有胜局',
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

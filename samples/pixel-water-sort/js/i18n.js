// Pixel Water Sort - localization (English / 中文)
const LANG_KEY = 'pixel-water-sort-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'WATER SORT',
    subtitle: 'Pour the colours until every tube is sorted.',
    howto: 'Tap a tube, then tap another to pour its top colour across. Each tube must end as one colour.',
    play: 'PLAY', levelSelect: 'LEVELS', menu: 'MENU', retry: 'RETRY', next: 'NEXT',
    level: 'LEVEL',
    win: 'ALL SORTED!', perfect: 'FLAWLESS!',
    winLine: (mv, ud) => `${mv} moves · ${ud} undos`,
    locked: 'LOCKED',
  },
  zh: {
    title: '颜色分类',
    subtitle: '倒水分类,直到每根试管只剩一种颜色。',
    howto: '点一根试管,再点另一根,把顶部颜色倒过去。每根试管最终都要是单一颜色。',
    play: '开始', levelSelect: '选关', menu: '菜单', retry: '重玩', next: '下一关',
    level: '关卡',
    win: '全部分类完成！', perfect: '完美无误！',
    winLine: (mv, ud) => `${mv} 步 · 撤销 ${ud} 次`,
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

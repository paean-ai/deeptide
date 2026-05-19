// Pixel Kakuro - English / Chinese strings.

const I18N = {
  en: {
    title: 'KAKURO',
    tagline: 'Fill the runs — every sum, no repeats',
    play: 'PLAY',
    pick: 'SELECT A PUZZLE',
    level: 'PUZZLE',
    menu: 'MENU', restart: 'RESTART', erase: 'ERASE',
    win: 'SOLVED!',
    winLine: 'Every run adds up.',
    next: 'NEXT', locked: 'LOCKED',
    howto: 'Fill white cells 1-9 so each across / down run hits its clue with no repeated digit.',
  },
  zh: {
    title: '数和',
    tagline: '填满每段——凑准和、不重复',
    play: '开始',
    pick: '选择谜题',
    level: '谜题',
    menu: '菜单', restart: '重来', erase: '擦除',
    win: '完成！',
    winLine: '每一段都凑齐了。',
    next: '下一关', locked: '未解锁',
    howto: '在白格填入 1-9，使每段横向 / 纵向之和等于提示，且数字不重复。',
  },
};

let lang = 'en';
try {
  const s = localStorage.getItem('pixel-kakuro-lang');
  if (s === 'en' || s === 'zh') lang = s;
} catch (e) { /* ignore */ }

function t(key, ...args) {
  const v = I18N[lang][key];
  return typeof v === 'function' ? v(...args) : v;
}

function setupLanguageToggle(onChange) {
  const btn = document.getElementById('btn-lang');
  if (!btn) return;
  const sync = () => { btn.textContent = lang === 'en' ? '中文' : 'EN'; };
  sync();
  btn.addEventListener('click', () => {
    lang = lang === 'en' ? 'zh' : 'en';
    try { localStorage.setItem('pixel-kakuro-lang', lang); } catch (e) { /* ignore */ }
    sync();
    if (onChange) onChange();
  });
}

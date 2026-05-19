// Pixel Laser Maze - localization (English / 中文)
const LANG_KEY = 'pixel-laser-maze-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'LASER MAZE',
    subtitle: 'Bend the beam through every crystal.',
    howto: 'Tap a tile to drop or rotate a mirror. Steer the laser so it passes through all the crystals.',
    play: 'PLAY', levelSelect: 'LEVELS', menu: 'MENU', retry: 'RETRY', next: 'NEXT',
    level: 'LEVEL', mirrors: 'MIRRORS',
    win: 'BEAM ALIGNED!', perfect: 'CLEAN ALIGNMENT!',
    winLine: n => `Solved with ${n} mirrors`,
    locked: 'LOCKED',
  },
  zh: {
    title: '镜光迷宫',
    subtitle: '折射光束,穿过每一颗水晶。',
    howto: '点击格子放置或旋转镜子,引导激光穿过所有水晶。',
    play: '开始', levelSelect: '选关', menu: '菜单', retry: '重玩', next: '下一关',
    level: '关卡', mirrors: '镜子',
    win: '光束对齐！', perfect: '完美对齐！',
    winLine: n => `用 ${n} 面镜子解开`,
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

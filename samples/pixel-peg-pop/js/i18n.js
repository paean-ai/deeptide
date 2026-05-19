// Pixel Peg Pop - localization (English / 中文)
const LANG_KEY = 'pixel-peg-pop-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'PEG POP',
    subtitle: 'Aim, drop, and bounce. Pop every orange peg.',
    howto: 'Drag to aim the launcher, release to fire. The ball ricochets through the pegs — clear all the orange ones to win.',
    play: 'PLAY', levelSelect: 'LEVELS', menu: 'MENU', retry: 'RETRY', next: 'NEXT',
    level: 'LEVEL', orange: 'ORANGE',
    win: 'LEVEL CLEAR!', perfect: 'BULLSEYE!',
    lose: 'OUT OF BALLS', loseMsg: 'Some orange pegs are still standing. Try again.',
    winLine: (sc, bl) => `Score ${sc} · ${bl} balls left`,
    locked: 'LOCKED',
  },
  zh: {
    title: '弹珠消消',
    subtitle: '瞄准、投放、弹射,打掉所有橙色钉。',
    howto: '拖动瞄准发射器,松手发射。小球在钉子间弹跳 —— 打掉全部橙色钉即获胜。',
    play: '开始', levelSelect: '选关', menu: '菜单', retry: '重玩', next: '下一关',
    level: '关卡', orange: '橙钉',
    win: '过关！', perfect: '神准！',
    lose: '弹珠用尽', loseMsg: '还有橙色钉没打掉,再试一次吧。',
    winLine: (sc, bl) => `得分 ${sc} · 剩余 ${bl} 球`,
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

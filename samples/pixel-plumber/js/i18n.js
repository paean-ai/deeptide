// Pixel Plumber - localization (English / 中文)
const LANG_KEY = 'pixel-plumber-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'PLUMBER',
    subtitle: 'Lay the pipe before the water catches up.',
    howto: 'Tap a tile to drop the next pipe piece. Connect the source to the drain before the water leaks out.',
    play: 'PLAY', levelSelect: 'LEVELS', menu: 'MENU', retry: 'RETRY', next: 'NEXT',
    level: 'LEVEL',
    win: 'PIPELINE DONE!', perfect: 'FLAWLESS PIPELINE!',
    lose: 'LEAK!', loseMsg: 'The water ran into a dead end. Try again.',
    flowIn: s => 'FLOW ' + s + 's',
    flowing: 'FLOWING',
    winLine: ow => `Routed home · ${ow} pieces re-laid`,
    locked: 'LOCKED',
  },
  zh: {
    title: '管道工',
    subtitle: '在水追上来之前铺好管道。',
    howto: '点击格子放下下一节管道。在水漏出之前把水源接到排水口。',
    play: '开始', levelSelect: '选关', menu: '菜单', retry: '重玩', next: '下一关',
    level: '关卡',
    win: '管线接通！', perfect: '完美管线！',
    lose: '漏水了！', loseMsg: '水流到了死路。再试一次吧。',
    flowIn: s => '放水 ' + s + 's',
    flowing: '放水中',
    winLine: ow => `成功接通 · 重铺 ${ow} 节`,
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

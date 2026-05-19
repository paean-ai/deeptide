// Pixel Mine Sweeper - localization (English / 中文)
const LANG_KEY = 'pixel-mine-sweeper-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'MINE SWEEPER',
    subtitle: 'Clear the field without tripping a mine.',
    howto: 'Tap to dig. Each number counts the mines touching that tile. Flag every mine to clear the field.',
    tip: 'Tip: long-press (or right-click) a tile to flag it.',
    play: 'PLAY', levelSelect: 'LEVELS', menu: 'MENU', retry: 'RETRY', next: 'NEXT',
    level: 'LEVEL', mines: 'MINES',
    modeDig: 'MODE: DIG', modeFlag: 'MODE: FLAG', scan: 'SCAN',
    win: 'FIELD CLEARED!', perfect: 'EXPERT CLEAR!',
    lose: 'BOOM!', loseMsg: 'You stepped on a mine. Take another run at it.',
    locked: 'LOCKED',
    winLine: tm => `Cleared in ${tm}`,
    bestLine: tm => `Best time ${tm}`,
    noScans: 'NO SCANS',
  },
  zh: {
    title: '扫雷探险',
    subtitle: '清空雷区,别踩到地雷。',
    howto: '点击挖开格子。数字表示周围 8 格里的地雷数量。标记出所有地雷即可通关。',
    tip: '提示:长按(或右键)格子可以插旗标记。',
    play: '开始', levelSelect: '选关', menu: '菜单', retry: '重试', next: '下一关',
    level: '关卡', mines: '地雷',
    modeDig: '模式:挖开', modeFlag: '模式:插旗', scan: '探测',
    win: '雷区已清空！', perfect: '专家通关！',
    lose: '爆炸了！', loseMsg: '你踩到了地雷,再来一次吧。',
    locked: '未解锁',
    winLine: tm => `用时 ${tm}`,
    bestLine: tm => `最佳用时 ${tm}`,
    noScans: '无探测',
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

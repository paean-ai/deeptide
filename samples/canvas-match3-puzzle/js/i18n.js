// Canvas Match-3 Puzzle - localization (English / 中文)
const LANG_KEY = 'canvas-match3-puzzle-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'MATCH-3 PUZZLE',
    level: 'Level', score: 'Score', moves: 'Moves', target: 'Target',
    combo: 'Combo', best: 'Best', shuffle: 'Shuffle', restart: 'Restart',
    levelMsg: n => `LEVEL ${n}`,
    outOfMoves: 'OUT OF MOVES', noMatch: 'NO MATCH', shuffleMsg: 'SHUFFLE',
    gems: ['Aqua', 'Leaf', 'Sun', 'Ruby', 'Violet', 'Bloom'],
  },
  zh: {
    title: '三消方块',
    level: '关卡', score: '分数', moves: '步数', target: '目标',
    combo: '连击', best: '最高', shuffle: '洗牌', restart: '重新开始',
    levelMsg: n => `第 ${n} 关`,
    outOfMoves: '步数耗尽', noMatch: '无法消除', shuffleMsg: '已洗牌',
    gems: ['水蓝', '叶绿', '骄阳', '红宝石', '紫晶', '花漾'],
  },
};

function t(key, ...args) {
  const v = TEXT[currentLang][key] ?? TEXT.en[key] ?? key;
  return typeof v === 'function' ? v(...args) : v;
}
function mt(m) { return m ? t(m.k, ...(m.a || [])) : ''; }
function gemName(i) { return TEXT[currentLang].gems[i] ?? TEXT.en.gems[i] ?? '?'; }

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

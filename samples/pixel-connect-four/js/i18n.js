// Pixel Connect Four - localization (English / 中文)
const LANG_KEY = 'pixel-connect-four-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'CONNECT 4',
    subtitle: 'Drop discs, line up four, outwit the machine.',
    howto: 'Tap a column to drop your disc. First to line up four — across, down or diagonal — wins.',
    pickDiff: 'Choose a difficulty:',
    menu: 'MENU', again: 'PLAY AGAIN',
    yourTurn: 'YOUR TURN', aiTurn: 'THINKING…',
    win: 'YOU WIN!', lose: 'AI WINS', draw: 'A DRAW',
    winMsg: 'Four in a row — nicely played!',
    loseMsg: 'The machine connected four. Try again.',
    drawMsg: 'The board filled with no winner.',
    statsLine: (w, l, d) => `Record  ${w}W · ${l}L · ${d}D`,
    noStats: 'No games played yet',
  },
  zh: {
    title: '四子连珠',
    subtitle: '落子、连成四子,斗智斗勇胜过机器。',
    howto: '点击某一列落下你的棋子。率先连成四子(横、竖或斜)者获胜。',
    pickDiff: '选择难度:',
    menu: '菜单', again: '再来一局',
    yourTurn: '你的回合', aiTurn: '思考中……',
    win: '你赢了！', lose: '电脑获胜', draw: '平局',
    winMsg: '连成四子 —— 漂亮！',
    loseMsg: '机器连成了四子,再来一次吧。',
    drawMsg: '棋盘已满,无人获胜。',
    statsLine: (w, l, d) => `战绩  ${w}胜 · ${l}负 · ${d}平`,
    noStats: '还没有对局记录',
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

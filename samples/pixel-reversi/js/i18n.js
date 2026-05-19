// Pixel Reversi - localization (English / 中文)
const LANG_KEY = 'pixel-reversi-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'REVERSI',
    subtitle: 'Flank and flip — own the most discs.',
    howto: 'Place a disc so it traps a line of the opponent\'s between yours; every trapped disc flips to your colour.',
    pickDiff: 'Choose a difficulty:',
    menu: 'MENU', again: 'PLAY AGAIN',
    yourTurn: 'YOUR TURN', aiTurn: 'THINKING…',
    win: 'YOU WIN!', lose: 'AI WINS', draw: 'A DRAW',
    winMsg: (a, b) => `You finished ${a} to ${b}.`,
    loseMsg: (a, b) => `The AI finished ${a} to ${b}.`,
    drawMsg: 'A dead-even board — well matched.',
    passNote: who => (who === 'you' ? 'No move — you pass.' : 'No move — AI passes.'),
    statsLine: (w, l, d) => `Record  ${w}W · ${l}L · ${d}D`,
    noStats: 'No games played yet',
  },
  zh: {
    title: '黑白棋',
    subtitle: '夹击翻转 —— 占据最多的棋子。',
    howto: '落子时夹住对方成一条直线的棋子,被夹住的棋子全部翻成你的颜色。',
    pickDiff: '选择难度:',
    menu: '菜单', again: '再来一局',
    yourTurn: '你的回合', aiTurn: '思考中……',
    win: '你赢了！', lose: '电脑获胜', draw: '平局',
    winMsg: (a, b) => `你以 ${a} 比 ${b} 获胜。`,
    loseMsg: (a, b) => `电脑以 ${a} 比 ${b} 获胜。`,
    drawMsg: '棋盘势均力敌 —— 旗鼓相当。',
    passNote: who => (who === 'you' ? '无子可下 —— 你跳过。' : '无子可下 —— 电脑跳过。'),
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

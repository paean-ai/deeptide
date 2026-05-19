// Pixel Dots & Boxes - English / Chinese strings.

const I18N = {
  en: {
    title: 'DOTS & BOXES',
    tagline: 'Close a box, take another turn',
    play: 'PLAY',
    difficulty: 'DIFFICULTY',
    you: 'YOU', cpu: 'CPU',
    yourTurn: 'YOUR TURN', cpuTurn: 'CPU THINKING',
    menu: 'MENU', again: 'PLAY AGAIN',
    win: 'YOU WIN!', lose: 'CPU WINS', draw: 'A DRAW',
    record: 'RECORD',
    result: (p, a) => 'You ' + p + '  ·  CPU ' + a,
    rec: (w, l, d) => w + 'W  ' + l + 'L  ' + d + 'D',
  },
  zh: {
    title: '点格棋',
    tagline: '围成方格即可再走一步',
    play: '开始',
    difficulty: '难度',
    you: '你', cpu: '电脑',
    yourTurn: '你的回合', cpuTurn: '电脑思考中',
    menu: '菜单', again: '再来一局',
    win: '你赢了！', lose: '电脑获胜', draw: '平局',
    record: '战绩',
    result: (p, a) => '你 ' + p + '  ·  电脑 ' + a,
    rec: (w, l, d) => w + '胜  ' + l + '负  ' + d + '平',
  },
};

let lang = 'en';
try {
  const s = localStorage.getItem('pixel-dots-boxes-lang');
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
    try { localStorage.setItem('pixel-dots-boxes-lang', lang); } catch (e) { /* ignore */ }
    sync();
    if (onChange) onChange();
  });
}

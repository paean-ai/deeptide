// Pixel Rally - English / Chinese strings.

const I18N = {
  en: {
    title: 'RALLY',
    tagline: 'Drag your paddle — first to 5',
    play: 'PLAY',
    pick: 'SELECT AN OPPONENT',
    level: 'BOUT',
    menu: 'MENU', restart: 'RESTART',
    win: 'MATCH WON!', lose: 'MATCH LOST',
    winLine: (p, c) => p + ' to ' + c + '.',
    next: 'NEXT', retry: 'RETRY',
    serve: 'SERVE',
    howto: 'Drag anywhere along the bottom to move your paddle. Hit the ball off-centre to angle the return — first to 5 points wins the match.',
  },
  zh: {
    title: '回合赛',
    tagline: '拖动球拍——先得 5 分获胜',
    play: '开始',
    pick: '选择对手',
    level: '对局',
    menu: '菜单', restart: '重来',
    win: '比赛胜利！', lose: '比赛失利',
    winLine: (p, c) => p + ' : ' + c + '。',
    next: '下一关', retry: '重试',
    serve: '发球',
    howto: '在屏幕下方拖动以移动球拍。让球偏离拍子中心可让回球带角度——先得 5 分赢得比赛。',
  },
};

let lang = 'en';
try {
  const s = localStorage.getItem('pixel-rally-lang');
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
    try { localStorage.setItem('pixel-rally-lang', lang); } catch (e) { /* ignore */ }
    sync();
    if (onChange) onChange();
  });
}

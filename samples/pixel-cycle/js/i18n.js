// Pixel Cycle - English / Chinese strings.

const I18N = {
  en: {
    title: 'CYCLE',
    tagline: 'Light-cycle duel — first to 2 round wins',
    play: 'PLAY',
    pick: 'SELECT AN OPPONENT',
    level: 'DUEL',
    menu: 'MENU', restart: 'RESTART',
    win: 'MATCH WON!', lose: 'MATCH LOST',
    winLine: (p, c) => 'Rounds ' + p + ' - ' + c + '.',
    next: 'NEXT', retry: 'RETRY',
    you: 'YOU', cpu: 'CPU',
    howto: 'Tap LEFT / RIGHT to turn your cycle. Trails are walls — drive into ANY trail (yours, theirs, or the edge) and you crash. Win 2 rounds to clear the duel.',
  },
  zh: {
    title: '光轮',
    tagline: '光轮对决——先赢两局者胜',
    play: '开始',
    pick: '选择对手',
    level: '对战',
    menu: '菜单', restart: '重来',
    win: '比赛胜利！', lose: '比赛失利',
    winLine: (p, c) => '比分 ' + p + ' - ' + c + '。',
    next: '下一关', retry: '重试',
    you: '你', cpu: '电脑',
    howto: '点击 LEFT / RIGHT 转向。光轮留下光墙——碰到任何光墙或边缘都会炸毁。先赢两局即通关。',
  },
};

let lang = 'en';
try {
  const s = localStorage.getItem('pixel-cycle-lang');
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
    try { localStorage.setItem('pixel-cycle-lang', lang); } catch (e) { /* ignore */ }
    sync();
    if (onChange) onChange();
  });
}

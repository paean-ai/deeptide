// Pixel Puck - English / Chinese strings.

const I18N = {
  en: {
    title: 'PUCK',
    tagline: 'Drag the mallet — slam it home',
    play: 'PLAY',
    pick: 'SELECT AN OPPONENT',
    level: 'MATCH',
    menu: 'MENU', restart: 'RESTART',
    win: 'MATCH WON!', lose: 'MATCH LOST',
    winLine: (p, c) => 'Final score ' + p + ' - ' + c + '.',
    next: 'NEXT', retry: 'RETRY',
    serve: 'SERVE',
    howto: 'Drag your finger anywhere in the lower half to move your mallet. Push the puck through the opponent goal at the top. First to 5 wins.',
  },
  zh: {
    title: '冰球',
    tagline: '拖动球拍——把球砸入对方球门',
    play: '开始',
    pick: '选择对手',
    level: '对局',
    menu: '菜单', restart: '重来',
    win: '比赛胜利！', lose: '比赛失利',
    winLine: (p, c) => '最终比分 ' + p + ' - ' + c + '。',
    next: '下一关', retry: '重试',
    serve: '发球',
    howto: '在下半场任意位置拖动手指来移动球拍。把冰球击入顶部对方球门——先得 5 分获胜。',
  },
};

let lang = 'en';
try {
  const s = localStorage.getItem('pixel-puck-lang');
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
    try { localStorage.setItem('pixel-puck-lang', lang); } catch (e) { /* ignore */ }
    sync();
    if (onChange) onChange();
  });
}

// Pixel Vanguard - English / Chinese strings.

const I18N = {
  en: {
    title: 'VANGUARD',
    tagline: 'Read the strikes — move, hit, and push them aside',
    play: 'PLAY',
    pick: 'SELECT A MISSION',
    level: 'OP',
    menu: 'MENU', restart: 'RESTART', endTurn: 'END TURN',
    win: 'AREA SECURED',
    winLine: 'Every hostile down.',
    lose: 'GRID DOWN',
    loseLine: 'The core fell.',
    retry: 'RETRY', next: 'NEXT',
    core: 'CORE',
    howto: 'Tap a mech, move it, then strike an adjacent enemy — every hit knocks the target back one tile. Red tiles show where enemies will strike next turn. Push them off your buildings.',
  },
  zh: {
    title: '先锋队',
    tagline: '看清攻击预警——移动、打击、并将敌人击退',
    play: '开始',
    pick: '选择任务',
    level: '行动',
    menu: '菜单', restart: '重来', endTurn: '结束回合',
    win: '区域肃清',
    winLine: '敌人已全部清除。',
    lose: '电网崩溃',
    loseLine: '核心被摧毁了。',
    retry: '重试', next: '下一关',
    core: '核心',
    howto: '点击机甲，移动它，再攻击相邻的敌人——每次命中都会把目标击退一格。红色格子是敌人下回合的攻击点，把它们推离你的建筑。',
  },
};

let lang = 'en';
try {
  const s = localStorage.getItem('pixel-vanguard-lang');
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
    try { localStorage.setItem('pixel-vanguard-lang', lang); } catch (e) { /* ignore */ }
    sync();
    if (onChange) onChange();
  });
}

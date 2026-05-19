// Pixel Circuit - English / Chinese strings.

const I18N = {
  en: {
    title: 'CIRCUIT',
    tagline: 'Rotate the wires — power every node',
    play: 'PLAY',
    pick: 'SELECT A BOARD',
    level: 'BOARD',
    menu: 'MENU', restart: 'RESTART',
    win: 'POWERED!', perfect: 'CLEAN ROUTE!',
    winLine: (mv, par) => mv + ' rotations — par ' + par + '.',
    next: 'NEXT',
    moves: 'TURNS', par: 'PAR',
    howto: 'Tap a tile to rotate the wires inside it. Connect every node to the power cell with no loose ends. Match par for the full stars.',
  },
  zh: {
    title: '电路板',
    tagline: '旋转连线——点亮所有节点',
    play: '开始',
    pick: '选择电路板',
    level: '电路',
    menu: '菜单', restart: '重来',
    win: '通电！', perfect: '完美布线！',
    winLine: (mv, par) => mv + ' 次旋转 — 标准 ' + par + ' 次。',
    next: '下一关',
    moves: '次数', par: '标准',
    howto: '点击格子旋转其中的连线。将所有节点连接到电源，且不留断头。达到标准次数可获满星。',
  },
};

let lang = 'en';
try {
  const s = localStorage.getItem('pixel-circuit-lang');
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
    try { localStorage.setItem('pixel-circuit-lang', lang); } catch (e) { /* ignore */ }
    sync();
    if (onChange) onChange();
  });
}

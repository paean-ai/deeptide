// Pixel Keg - English / Chinese strings.

const I18N = {
  en: {
    title: 'KEG',
    tagline: 'Crack the bricks, dodge the blasts, find the stairs',
    play: 'PLAY',
    pick: 'SELECT A FLOOR',
    level: 'FLOOR',
    menu: 'MENU', restart: 'RESTART',
    win: 'STAIRS REACHED!', lose: 'BLOWN AWAY',
    winLine: lv => 'Floor ' + (lv + 1) + ' cleared.',
    loseLine: 'The dungeon claimed you.',
    next: 'NEXT', retry: 'RETRY',
    howto: 'Tap the arrows to move, BOMB to drop a powder keg. The fuse runs out in two seconds — blast bricks to find the stairs, but stand clear of the flames.',
  },
  zh: {
    title: '火药桶',
    tagline: '炸开砖块，躲开火焰，找到下楼通道',
    play: '开始',
    pick: '选择楼层',
    level: '楼层',
    menu: '菜单', restart: '重来',
    win: '到达通道！', lose: '被炸飞了',
    winLine: lv => '通过第 ' + (lv + 1) + ' 层。',
    loseLine: '地牢吞噬了你。',
    next: '下一关', retry: '重试',
    howto: '点击方向移动，BOMB 投下火药桶。引信约两秒——炸开砖块寻找下楼通道，但务必避开火焰。',
  },
};

let lang = 'en';
try {
  const s = localStorage.getItem('pixel-keg-lang');
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
    try { localStorage.setItem('pixel-keg-lang', lang); } catch (e) { /* ignore */ }
    sync();
    if (onChange) onChange();
  });
}

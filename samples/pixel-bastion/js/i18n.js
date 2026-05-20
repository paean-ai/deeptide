// Pixel Bastion - English / Chinese strings.

const I18N = {
  en: {
    title: 'BASTION',
    tagline: 'Tap the sky — defend the cities',
    play: 'PLAY',
    pick: 'SELECT A WAVE',
    level: 'WAVE',
    menu: 'MENU', restart: 'RESTART',
    win: 'AREA SECURED', lose: 'CITIES LOST',
    winLine: (cities, score) => cities + '/5 cities  ·  ' + score + ' pts.',
    loseLine: 'The bastion has fallen.',
    next: 'NEXT', retry: 'RETRY',
    score: 'SCORE',
    howto: 'Tap anywhere in the sky to fire a counter-missile from the nearest silo with ammo. The explosion knocks down any incoming missile inside the blast. Keep at least one city standing.',
  },
  zh: {
    title: '堡垒',
    tagline: '点击天空——守护城市',
    play: '开始',
    pick: '选择波次',
    level: '波次',
    menu: '菜单', restart: '重来',
    win: '区域保住了', lose: '城市陨落',
    winLine: (cities, score) => cities + '/5 座城市 · ' + score + ' 分。',
    loseLine: '堡垒已经陷落。',
    next: '下一关', retry: '重试',
    score: '得分',
    howto: '点击天空任意位置，最近的有弹药发射井便会射出拦截弹。爆炸可击落进入范围的来袭导弹。至少保住一座城市。',
  },
};

let lang = 'en';
try {
  const s = localStorage.getItem('pixel-bastion-lang');
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
    try { localStorage.setItem('pixel-bastion-lang', lang); } catch (e) { /* ignore */ }
    sync();
    if (onChange) onChange();
  });
}

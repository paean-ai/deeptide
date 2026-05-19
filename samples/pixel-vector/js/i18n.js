// Pixel Vector - English / Chinese strings.

const I18N = {
  en: {
    title: 'VECTOR',
    tagline: 'Drift, rotate, fire — clear the asteroid field',
    play: 'PLAY',
    pick: 'SELECT A SECTOR',
    level: 'SECTOR',
    menu: 'MENU', restart: 'RESTART',
    win: 'FIELD CLEARED!', lose: 'SHIP LOST',
    winLine: sc => 'Score ' + sc + '.',
    loseLine: 'All ships destroyed.',
    next: 'NEXT', retry: 'RETRY',
    score: 'SCORE',
    howto: 'Hold ROTATE arrows to turn, THRUST to fly, FIRE to shoot. Asteroids split when hit. Watch the wrap-around edges — you can drift right off the screen and back on the other side.',
  },
  zh: {
    title: '矢量',
    tagline: '漂流、转向、开火——清空小行星带',
    play: '开始',
    pick: '选择星区',
    level: '星区',
    menu: '菜单', restart: '重来',
    win: '区域肃清！', lose: '飞船陨落',
    winLine: sc => '得分 ' + sc + '。',
    loseLine: '所有飞船都被击毁。',
    next: '下一关', retry: '重试',
    score: '得分',
    howto: '按住旋转箭头转向，THRUST 推进，FIRE 射击。陨石被击中会分裂。边缘会环绕——飞出一边会从另一边重新进入。',
  },
};

let lang = 'en';
try {
  const s = localStorage.getItem('pixel-vector-lang');
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
    try { localStorage.setItem('pixel-vector-lang', lang); } catch (e) { /* ignore */ }
    sync();
    if (onChange) onChange();
  });
}

// Pixel Flap - English / Chinese strings.

const I18N = {
  en: {
    title: 'FLAP',
    tagline: 'Tap to flap — slip through the gaps',
    play: 'PLAY',
    pick: 'SELECT A SKY',
    level: 'SKY',
    menu: 'MENU', restart: 'RESTART',
    win: 'CLEARED!', lose: 'GAME OVER',
    winLine: count => 'Passed ' + count + ' pipes.',
    loseLine: (passed, target) => 'Passed ' + passed + ' of ' + target + '.',
    next: 'NEXT', retry: 'RETRY',
    tapStart: 'TAP TO FLAP',
    howto: 'Tap anywhere to flap upward. Gravity does the rest — slip through every pipe gap. Pass the target number of pipes to clear the level.',
  },
  zh: {
    title: '振翅',
    tagline: '点击振翅——穿过空隙',
    play: '开始',
    pick: '选择天空',
    level: '天空',
    menu: '菜单', restart: '重来',
    win: '通关！', lose: '游戏结束',
    winLine: count => '通过 ' + count + ' 根管道。',
    loseLine: (passed, target) => '通过 ' + passed + ' / ' + target + '。',
    next: '下一关', retry: '重试',
    tapStart: '点击振翅',
    howto: '点击屏幕任意位置振翅升起，重力会让你下坠。穿过每根管道的空隙，通过目标数量即可通关。',
  },
};

let lang = 'en';
try {
  const s = localStorage.getItem('pixel-flap-lang');
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
    try { localStorage.setItem('pixel-flap-lang', lang); } catch (e) { /* ignore */ }
    sync();
    if (onChange) onChange();
  });
}

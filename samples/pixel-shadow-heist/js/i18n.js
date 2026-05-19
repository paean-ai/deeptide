// Pixel Shadow Heist - localization (English / 中文)
const LANG_KEY = 'pixel-shadow-heist-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'SHADOW HEIST',
    subtitle: 'Slip past the guards. One move at a time.',
    play: 'PLAY', levelSelect: 'LEVELS',
    howto: 'Arrows / WASD or swipe to move. Tap WAIT to let a guard pass.',
    level: 'Level', turns: 'Turns', caught: 'Caught', best: 'Best',
    wait: 'Wait', restart: 'Restart', menu: 'Menu', next: 'NEXT JOB',
    paused: 'PAUSED', resume: 'Resume',
    cleared: 'VAULT REACHED', clearedMsg: (turn, c) => `Done in ${turn} turns, caught ${c}x.`,
    spotted: 'SPOTTED!', allDone: 'MASTER THIEF', allDoneMsg: 'Every vault cracked. Flawless work.',
    bestLine: t => `Best: ${t} turns`, locked: 'Locked',
    tip: 'Guards only see straight ahead. Cross behind their backs.',
  },
  zh: {
    title: '暗影行窃',
    subtitle: '避开守卫，一步一步潜入。',
    play: '开始', levelSelect: '选关',
    howto: '方向键 / WASD 或滑动移动。点「等待」让守卫走过。',
    level: '关卡', turns: '步数', caught: '被发现', best: '最佳',
    wait: '等待', restart: '重来', menu: '菜单', next: '下一票',
    paused: '已暂停', resume: '继续',
    cleared: '抵达金库', clearedMsg: (turn, c) => `用 ${turn} 步完成，被发现 ${c} 次。`,
    spotted: '被发现了！', allDone: '神偷大师', allDoneMsg: '所有金库已破解，完美无瑕。',
    bestLine: t => `最佳：${t} 步`, locked: '未解锁',
    tip: '守卫只能看到正前方，从他们背后穿过。',
  },
};

function t(key, ...args) {
  const v = TEXT[currentLang][key] ?? TEXT.en[key] ?? key;
  return typeof v === 'function' ? v(...args) : v;
}
function applyStaticText() {
  document.documentElement.lang = currentLang === 'zh' ? 'zh-CN' : 'en';
  document.querySelectorAll('[data-i18n]').forEach(el => { el.textContent = t(el.dataset.i18n); });
}
function setupLanguageToggle(onChange) {
  const btn = document.getElementById('lang-toggle');
  if (btn) {
    btn.onclick = () => {
      currentLang = currentLang === 'en' ? 'zh' : 'en';
      localStorage.setItem(LANG_KEY, currentLang);
      btn.textContent = currentLang === 'en' ? '中文' : 'EN';
      applyStaticText();
      if (onChange) onChange();
    };
    btn.textContent = currentLang === 'en' ? '中文' : 'EN';
  }
  applyStaticText();
}

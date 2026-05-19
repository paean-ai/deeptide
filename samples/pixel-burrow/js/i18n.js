// Pixel Burrow - English / Chinese strings.

const I18N = {
  en: {
    title: 'BURROW',
    tagline: 'Bonk the critters — never the bombs',
    play: 'PLAY',
    pick: 'SELECT A FIELD',
    level: 'FIELD',
    menu: 'MENU', restart: 'RESTART',
    score: 'SCORE', goal: 'GOAL', time: 'TIME',
    win: 'FIELD CLEARED!', lose: 'TIME UP',
    bust: 'BLOWN UP!',
    winLine: sc => 'You scored ' + sc + '.',
    loseLine: (sc, tg) => 'Scored ' + sc + ' of ' + tg + '.',
    next: 'NEXT', retry: 'RETRY',
    combo: 'COMBO',
    howto: 'Tap the gophers as they pop up — golden ones are worth far more. Never tap a bomb: it costs a life. Beat the score goal before time runs out.',
  },
  zh: {
    title: '地洞',
    tagline: '敲打地鼠——千万别敲炸弹',
    play: '开始',
    pick: '选择田地',
    level: '田地',
    menu: '菜单', restart: '重来',
    score: '得分', goal: '目标', time: '时间',
    win: '通过！', lose: '时间到',
    bust: '被炸飞了！',
    winLine: sc => '你得了 ' + sc + ' 分。',
    loseLine: (sc, tg) => '得分 ' + sc + ' / ' + tg + '。',
    next: '下一关', retry: '重试',
    combo: '连击',
    howto: '地鼠冒头时点击它——金色地鼠分数高得多。切勿点击炸弹，会损失一条命。在时间结束前达到目标分数。',
  },
};

let lang = 'en';
try {
  const s = localStorage.getItem('pixel-burrow-lang');
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
    try { localStorage.setItem('pixel-burrow-lang', lang); } catch (e) { /* ignore */ }
    sync();
    if (onChange) onChange();
  });
}

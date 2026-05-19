// Pixel Putt Quest - localization (English / 中文)
const LANG_KEY = 'pixel-putt-quest-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'PUTT QUEST',
    subtitle: 'Drag back, aim, and sink it under par.',
    play: 'PLAY', levelSelect: 'HOLES', howto: 'Drag from the ball to aim and set power, release to putt.',
    hole: 'Hole', par: 'Par', strokes: 'Strokes', total: 'Total',
    restart: 'Replay', menu: 'Menu', next: 'NEXT HOLE',
    sunk: 'IN THE CUP!', holeInOne: 'HOLE IN ONE!',
    underPar: n => `${n} under par`, overPar: n => `${n} over par`, onPar: 'Even par',
    splash: 'SPLASH! +1', allDone: 'COURSE COMPLETE',
    allDoneMsg: t => `Round finished at ${t}. Tidy putting!`,
    best: t => `Best total: ${t}`, locked: 'Locked',
  },
  zh: {
    title: '推杆征程',
    subtitle: '后拉蓄力、瞄准,低于标准杆进洞。',
    play: '开始', levelSelect: '选洞', howto: '从球上向后拖动来瞄准并蓄力,松手击球。',
    hole: '球洞', par: '标准杆', strokes: '杆数', total: '总杆',
    restart: '重打', menu: '菜单', next: '下一洞',
    sunk: '进洞！', holeInOne: '一杆进洞！',
    underPar: n => `低于标准杆 ${n} 杆`, overPar: n => `高于标准杆 ${n} 杆`, onPar: '标准杆',
    splash: '入水！+1', allDone: '完成全场',
    allDoneMsg: t => `全场总杆 ${t},推杆漂亮！`,
    best: t => `最佳总杆：${t}`, locked: '未解锁',
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

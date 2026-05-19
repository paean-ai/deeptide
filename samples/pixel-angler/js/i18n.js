// Pixel Angler - localization (English / 中文)
const LANG_KEY = 'pixel-angler-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'ANGLER',
    subtitle: 'Cast, hook, and reel in the big one.',
    howto: 'Cast your line, tap the instant a fish bites, then hold to keep the catch bar over the fish until it\'s landed.',
    play: 'GO FISHING', menu: 'MENU', shop: 'TACKLE SHOP', back: 'BACK',
    castMsg: 'Tap CAST to throw your line', waiting: 'Waiting for a bite…',
    bite: 'BITE!  TAP!', reelMsg: 'HOLD to reel — keep the bar on the fish',
    caught: n => `Landed a ${n}!`, escaped: 'It got away…', missed: 'Too slow — it spat the hook!',
    rod: 'Rod', reel: 'Reel', rodDesc: 'Wider catch bar', reelDesc: 'Faster reel-in',
    unlock: 'Unlock', maxed: 'MAXED', owned: 'OPEN',
    coins: c => '◎ ' + c, coinsLine: c => `${c} coins earned`,
  },
  zh: {
    title: '垂钓大师',
    subtitle: '抛竿、上钩,把大鱼拉上岸。',
    howto: '抛出鱼线,鱼咬钩的瞬间点击,然后按住让收线条始终罩住鱼,直到把它钓上来。',
    play: '去钓鱼', menu: '菜单', shop: '渔具店', back: '返回',
    castMsg: '点击「抛竿」甩出鱼线', waiting: '等待鱼儿上钩……',
    bite: '咬钩了！快点！', reelMsg: '按住收线 —— 让线条罩住鱼',
    caught: n => `钓到了${n}！`, escaped: '鱼跑掉了……', missed: '太慢了 —— 鱼把钩吐了！',
    rod: '鱼竿', reel: '渔轮', rodDesc: '更宽的收线条', reelDesc: '更快的收线速度',
    unlock: '解锁', maxed: '已满级', owned: '已开放',
    coins: c => '◎ ' + c, coinsLine: c => `已赚取 ${c} 金币`,
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

// Pixel Merge Garden - localization (English / 中文)
const LANG_KEY = 'pixel-merge-garden-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'MERGE GARDEN',
    coins: 'Coins', bank: 'Bank', best: 'Best Crop', income: 'Income', order: 'Order',
    buySeed: 'Buy Seed', collect: 'Collect', water: 'Water', upgrade: 'Greenhouse', reset: 'Reset',
    howto: 'Drag a crop onto a matching one to merge — or tap two in turn.',
    seed: 'SEED', silverSeed: 'SILVER SEED', wildSeed: 'WILD SEED',
    merge: 'MERGE', order_: 'ORDER', rainBoost: 'RAIN BOOST', collectF: 'COLLECT',
    greenhouseUp: 'GREENHOUSE LV',
    combo: 'COMBO', maxGreenhouse: 'MAX',
    rain: 'rain', perSec: '/s',
    welcomeBack: 'WELCOME BACK',
    offline: n => `Garden earned ${n} while you were away`,
    confirmReset: 'Reset all garden progress?',
    wildHint: 'Wild crop — merges with anything!',
  },
  zh: {
    title: '合成花园',
    coins: '金币', bank: '存钱罐', best: '最高作物', income: '收入', order: '订单',
    buySeed: '买种子', collect: '收取', water: '浇水', upgrade: '温室', reset: '重置',
    howto: '把作物拖到相同的作物上即可合成 —— 或依次点选两个。',
    seed: '种子', silverSeed: '银种子', wildSeed: '万能种子',
    merge: '合成', order_: '订单', rainBoost: '降雨加成', collectF: '收取',
    greenhouseUp: '温室升至 LV',
    combo: '连击', maxGreenhouse: '满级',
    rain: '雨', perSec: '/秒',
    welcomeBack: '欢迎回来',
    offline: n => `离开期间花园收获了 ${n}`,
    confirmReset: '确定要重置全部花园进度吗？',
    wildHint: '万能作物 —— 可与任意作物合成！',
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

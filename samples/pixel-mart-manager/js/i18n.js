// Pixel Mart Manager - localization (English / 中文)
const LANG_KEY = 'pixel-mart-manager-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'PIXEL MART',
    coins: 'Coins', stock: 'Stock', shelf: 'Shelf', rep: 'Rep',
    upgradeShelf: 'Upgrade Shelf', hireHelper: 'Hire Helper',
    expandMart: 'Expand Mart', marketing: 'Marketing',
    zoneField: 'BANANA GROVE', zoneShelf: 'SHELF', zoneRegister: 'CHECKOUT',
    stockGain: n => `+${n} stock`, shelfGain: n => `shelf +${n}`,
    coinsGain: n => `+${n} coins`, missed: 'missed sale',
    shelfUp: 'shelf upgraded', helperUp: 'helper hired',
    martUp: 'mart expanded', marketUp: 'marketing boosted',
    welcomeBack: n => `Welcome back — helpers earned +${n}`,
    hint: 'Move to a zone and press ACT / Space: gather, restock, checkout.',
  },
  zh: {
    title: '像素超市',
    coins: '金币', stock: '库存', shelf: '货架', rep: '口碑',
    upgradeShelf: '升级货架', hireHelper: '雇佣店员',
    expandMart: '扩张超市', marketing: '营销推广',
    zoneField: '香蕉园', zoneShelf: '货架', zoneRegister: '收银台',
    stockGain: n => `+${n} 库存`, shelfGain: n => `货架 +${n}`,
    coinsGain: n => `+${n} 金币`, missed: '错失订单',
    shelfUp: '货架已升级', helperUp: '已雇佣店员',
    martUp: '超市已扩张', marketUp: '营销已加强',
    welcomeBack: n => `欢迎回来 —— 店员赚到了 +${n}`,
    hint: '走到区域并按 ACT / 空格：采摘、补货、收银。',
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

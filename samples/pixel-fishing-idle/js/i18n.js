// Pixel Fishing Idle - localization (English / 中文)
const LANG_KEY = 'pixel-fishing-idle-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'PIXEL FISHING',
    coins: 'Coins', rod: 'Rod', bait: 'Bait', boat: 'Boat', zone: 'Zone', order: 'Order',
    castLine: 'Cast Line', reelIn: 'Reel In!', pullBack: 'Pull Back',
    upRod: 'Rod', upBait: 'Bait', upBoat: 'Boat', upCrew: 'Crew', sail: 'Sail',
    weatherCalm: 'Calm', weatherStorm: 'Storm', weatherLucky: 'Lucky Tide',
    howStart: 'Cast, stop the marker in the green band, then reel.',
    castIn: (z, w) => `${z}: line cast in ${w.toLowerCase()}.`,
    bite: 'BITE! Stop the marker in the green band!',
    early: 'Pulled too early. The line went quiet.',
    tooLate: 'Too late — the fish shook free.',
    caught: (q, n, r, v) => `${q} ${n} (${r}). +${v}`,
    perfect: 'Perfect catch!', good: 'Caught', miss: 'Slipped —',
    crewHaul: (n, v) => `Crew hauled ${n}. +${v}`,
    orderDone: (n, r) => `Order complete: ${n}. +${r}`,
    upgraded: { rod: 'Rod upgraded: faster bites, wider band, higher value.',
      bait: 'Bait upgraded: rarer fish surface more often.',
      boat: 'Boat upgraded: new waters and stronger idle income.',
      crew: 'Crew upgraded: idle hauls improved.' },
    needBoat: (z, n) => `${z} needs boat ${n}.`,
    sailed: (z, w) => `Sailed to ${z}. Weather: ${w}.`,
    welcomeBack: v => `Welcome back — your crew hauled +${v} while away.`,
    quality: { perfect: 'PERFECT', good: 'GOOD', miss: 'MISS' },
  },
  zh: {
    title: '像素钓鱼',
    coins: '金币', rod: '钓竿', bait: '鱼饵', boat: '渔船', zone: '海域', order: '订单',
    castLine: '抛竿', reelIn: '收线！', pullBack: '收回',
    upRod: '钓竿', upBait: '鱼饵', upBoat: '渔船', upCrew: '船员', sail: '启航',
    weatherCalm: '平静', weatherStorm: '风暴', weatherLucky: '幸运潮',
    howStart: '抛竿，把指针停在绿色区域内，再收线。',
    castIn: (z, w) => `${z}：在${w}中抛下钓线。`,
    bite: '上钩了！把指针停在绿色区域！',
    early: '收线太早，鱼线归于平静。',
    tooLate: '太迟了 —— 鱼挣脱了。',
    caught: (q, n, r, v) => `${q} ${n}（${r}）。+${v}`,
    perfect: '完美捕获！', good: '钓到', miss: '溜走了 ——',
    crewHaul: (n, v) => `船员捞到 ${n}。+${v}`,
    orderDone: (n, r) => `订单完成：${n}。+${r}`,
    upgraded: { rod: '钓竿升级：上钩更快、绿区更宽、价值更高。',
      bait: '鱼饵升级：稀有鱼出现得更频繁。',
      boat: '渔船升级：解锁新海域，挂机收入更强。',
      crew: '船员升级：挂机捕捞更高效。' },
    needBoat: (z, n) => `${z} 需要渔船等级 ${n}。`,
    sailed: (z, w) => `启航前往${z}。天气：${w}。`,
    welcomeBack: v => `欢迎回来 —— 离开期间船员捞到 +${v}。`,
    quality: { perfect: '完美', good: '不错', miss: '失手' },
  },
};

// fish + zone + rarity display names
const NAMES = {
  en: {
    fish: { minnow: 'Glass Minnow', carp: 'Copper Carp', crab: 'Pebble Crab', koi: 'Lantern Koi',
      pike: 'Kelp Pike', perch: 'Reed Perch', eel: 'Emerald Eel', thorn: 'Thornfish',
      moonfin: 'Moonfin', jelly: 'Glow Jelly', tang: 'Prism Tang', ray: 'Star Ray',
      bass: 'Gilded Bass', lurker: 'Abyss Lurker', levi: 'Crown Levi',
      frostfin: 'Frostfin', crystal: 'Crystal Cod', icejaw: 'Icejaw',
      aurora: 'Aurora Ray', titan: 'Glacier Titan' },
    zone: { cove: 'Cove', kelp: 'Kelp Bay', reef: 'Moon Reef', crown: 'Sunken Crown', frost: 'Frost Tides' },
    rarity: { common: 'common', uncommon: 'uncommon', rare: 'rare', epic: 'epic', mythic: 'mythic' },
  },
  zh: {
    fish: { minnow: '玻璃鲦', carp: '铜鲤', crab: '卵石蟹', koi: '灯笼锦鲤',
      pike: '海带梭', perch: '芦苇鲈', eel: '翡翠鳗', thorn: '荆棘鱼',
      moonfin: '月鳍鱼', jelly: '辉光水母', tang: '棱镜刺尾鱼', ray: '星鳐',
      bass: '鎏金鲈', lurker: '深渊潜伏者', levi: '王冠利维坦',
      frostfin: '霜鳍', crystal: '水晶鳕', icejaw: '冰颚',
      aurora: '极光鳐', titan: '冰川泰坦' },
    zone: { cove: '海湾', kelp: '海带湾', reef: '月礁', crown: '沉没王冠', frost: '霜潮' },
    rarity: { common: '普通', uncommon: '少见', rare: '稀有', epic: '史诗', mythic: '神话' },
  },
};

function t(key, ...args) {
  const v = TEXT[currentLang][key] ?? TEXT.en[key] ?? key;
  return typeof v === 'function' ? v(...args) : v;
}
function tUp(key) { return TEXT[currentLang].upgraded[key]; }
function tQuality(q) { return TEXT[currentLang].quality[q]; }
function tFish(id) { return NAMES[currentLang].fish[id] || id; }
function tZone(id) { return NAMES[currentLang].zone[id] || id; }
function tRarity(r) { return NAMES[currentLang].rarity[r] || r; }
function tWeather(w) { return t('weather' + w); } // w: Calm/Storm/Lucky

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

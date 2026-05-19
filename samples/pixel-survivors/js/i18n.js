// Pixel Survivors - localization
const LANG_KEY = 'pixel-survivors-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'PIXEL SURVIVORS',
    subtitle: 'Auto-attack. Level up. Outlast the horde.',
    play: 'SURVIVE',
    armory: 'ARMORY',
    howto: 'Move to dodge — your weapons fire on their own. Collect gems to level up.',
    back: 'Back', bank: 'Bank',
    levelUp: 'LEVEL UP', chooseUpgrade: 'Choose an upgrade',
    newWeapon: 'New Weapon', newItem: 'New Item', evolved: 'EVOLVED!',
    maxed: 'MAX',
    paused: 'PAUSED', resume: 'Resume', quit: 'Quit', restart: 'Restart',
    gameOver: 'YOU FELL', victory: 'YOU SURVIVED',
    survivedFor: s => `Survived ${fmtTime(s)}`,
    killsLabel: 'Kills', levelLabel: 'Level', goldEarned: g => `Gold earned +${g}`,
    revived: 'REVIVED', goalReached: 'The Overlord approaches...',
    metaTitle: 'PERMANENT UPGRADES',
    buy: 'Buy', owned: 'Owned',
    hp: 'HP', lvl: 'Lv', time: 'Time', kills: 'Kills',
    weapons: 'Weapons', items: 'Items',
    metaNames: {
      power: 'Power', vigor: 'Vigor', speed: 'Swiftness', growth: 'Growth',
      fortune: 'Fortune', recovery: 'Recovery', revive: 'Second Wind',
    },
    metaDesc: {
      power: '+5% damage', vigor: '+14 max HP', speed: '+4% move speed',
      growth: '+8% XP gain', fortune: '+12% gold', recovery: '+0.4 HP/s regen',
      revive: 'Revive once per run',
    },
    wName: {
      dagger: 'Dagger', aura: 'Holy Aura', orbit: 'Orbit Blade', bolt: 'Chain Bolt',
      nova: 'Frost Nova', fireball: 'Fireball', shards: 'Shard Burst',
      boomerang: 'Boomerang', coil: 'Arc Coil', skyfall: 'Skyfall',
    },
    wEvo: {
      dagger: 'Thousand Knives', aura: 'Sanctuary', orbit: 'Halo',
      bolt: 'Tempest', nova: 'Blizzard', fireball: 'Meteor Storm',
      shards: 'Shard Nova', boomerang: 'Eternal Return', coil: 'Tesla Storm',
      skyfall: 'Armageddon',
    },
    wDesc: {
      dagger: 'Throws blades at the nearest foe.',
      aura: 'Burns enemies that draw near.',
      orbit: 'Blades circle and shield you.',
      bolt: 'Lightning leaps to random enemies.',
      nova: 'Frost burst that slows and shatters.',
      fireball: 'Lobbed bombs that explode on impact.',
      shards: 'Sprays cutting shards in every direction.',
      boomerang: 'A blade that sweeps out and curves back.',
      coil: 'Sparks chain through clustered foes.',
      skyfall: 'Meteors crash down from the sky.',
    },
    pName: {
      might: 'Might', swift: 'Swift Boots', haste: 'Haste', armor: 'Plate Armor',
      magnet: 'Lodestone', vitality: 'Vitality',
    },
    pDesc: {
      might: '+9% damage', swift: '+8% move speed', haste: '-8% cooldown',
      armor: 'Take less contact damage', magnet: '+30% pickup range', vitality: '+24 max HP',
    },
    lvlOf: (a) => `+1 → Lv.${a}`,
  },
  zh: {
    title: '像素幸存者',
    subtitle: '自动攻击，升级强化，撑过虫潮。',
    play: '开始求生',
    armory: '军械库',
    howto: '移动躲避——武器会自动开火。拾取经验宝石来升级。',
    back: '返回', bank: '金库',
    levelUp: '升级', chooseUpgrade: '选择一项强化',
    newWeapon: '新武器', newItem: '新道具', evolved: '已进化！',
    maxed: '满级',
    paused: '已暂停', resume: '继续', quit: '退出', restart: '重新开始',
    gameOver: '你倒下了', victory: '你活下来了',
    survivedFor: s => `存活 ${fmtTime(s)}`,
    killsLabel: '击杀', levelLabel: '等级', goldEarned: g => `获得金币 +${g}`,
    revived: '复活', goalReached: '霸主正在逼近……',
    metaTitle: '永久强化',
    buy: '购买', owned: '已拥有',
    hp: '生命', lvl: '等级', time: '时间', kills: '击杀',
    weapons: '武器', items: '道具',
    metaNames: {
      power: '力量', vigor: '活力', speed: '迅捷', growth: '成长',
      fortune: '财运', recovery: '回复', revive: '重整旗鼓',
    },
    metaDesc: {
      power: '伤害 +5%', vigor: '最大生命 +14', speed: '移速 +4%',
      growth: '经验获取 +8%', fortune: '金币 +12%', recovery: '每秒回血 +0.4',
      revive: '每局可复活一次',
    },
    wName: {
      dagger: '飞刀', aura: '圣光环', orbit: '环刃', bolt: '连锁闪电',
      nova: '寒霜新星', fireball: '火球', shards: '碎刃',
      boomerang: '回旋镖', coil: '电弧线圈', skyfall: '天陨',
    },
    wEvo: {
      dagger: '千刃乱舞', aura: '圣域', orbit: '星环',
      bolt: '暴风', nova: '暴雪', fireball: '陨石风暴',
      shards: '碎刃风暴', boomerang: '永恒回归', coil: '特斯拉风暴',
      skyfall: '末日审判',
    },
    wDesc: {
      dagger: '向最近的敌人投掷利刃。',
      aura: '灼烧靠近的敌人。',
      orbit: '环绕飞行的刀刃保护你。',
      bolt: '闪电跳跃攻击随机敌人。',
      nova: '减速并击碎敌人的霜爆。',
      fireball: '抛射并在落点爆炸的炸弹。',
      shards: '向四面八方喷射切割碎刃。',
      boomerang: '挥出后弧线飞回的利刃。',
      coil: '电流在密集的敌群中连锁跳跃。',
      skyfall: '陨石从天而降砸向敌群。',
    },
    pName: {
      might: '蛮力', swift: '疾行靴', haste: '急速', armor: '板甲',
      magnet: '磁石', vitality: '活力',
    },
    pDesc: {
      might: '伤害 +9%', swift: '移速 +8%', haste: '冷却 -8%',
      armor: '减少接触伤害', magnet: '拾取范围 +30%', vitality: '最大生命 +24',
    },
    lvlOf: (a) => `+1 → ${a}级`,
  },
};

function fmtTime(sec) {
  sec = Math.max(0, Math.floor(sec));
  const m = Math.floor(sec / 60), s = sec % 60;
  return m + ':' + String(s).padStart(2, '0');
}
function t(key, ...args) {
  const v = TEXT[currentLang][key] ?? TEXT.en[key] ?? key;
  return typeof v === 'function' ? v(...args) : v;
}
function tw(id, evolved) { return (evolved ? TEXT[currentLang].wEvo : TEXT[currentLang].wName)[id]; }
function twDesc(id) { return TEXT[currentLang].wDesc[id]; }
function tp(id) { return TEXT[currentLang].pName[id]; }
function tpDesc(id) { return TEXT[currentLang].pDesc[id]; }
function tMeta(id) { return TEXT[currentLang].metaNames[id]; }
function tMetaDesc(id) { return TEXT[currentLang].metaDesc[id]; }

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

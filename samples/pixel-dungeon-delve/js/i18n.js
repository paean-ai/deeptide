// Pixel Dungeon Delve - localization (English / 中文)
const LANG_KEY = 'pixel-dungeon-delve-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'DUNGEON DELVE',
    subtitle: 'A turn-based crawl into the dark. Every step is a turn.',
    play: 'DESCEND', resume: 'CONTINUE RUN', howto: 'Move with arrows / WASD or swipe. Bump foes to attack.',
    depth: 'Depth', floor: 'Floor', hp: 'HP', lvl: 'Lv', gold: 'Gold', xp: 'XP',
    weapon: 'Weapon', armor: 'Armor',
    paused: 'PAUSED', restart: 'New Run', menu: 'Menu', close: 'Resume',
    gameOver: 'YOU DIED', victory: 'THE WYRM FALLS',
    reachedDepth: d => `You fell on depth ${d}.`,
    victoryMsg: 'The dragon is slain — but the abyss runs deeper. Delve on.',
    bestDepth: d => `Deepest delve: ${d}`,
    again: 'DELVE AGAIN', keepGoing: 'KEEP DELVING',
    foundWeapon: n => `Found ${n}`, foundArmor: n => `Found ${n}`,
    quaff: 'Quaffed a potion', gotGold: g => `+${g} gold`,
    levelUp: l => `Level up! Lv.${l}`, descend: d => `You descend to depth ${d}.`,
    hitFor: (n, d) => `You hit ${n} for ${d}.`, slain: n => `${n} is slain!`,
    tookHit: (n, d) => `${n} hits you for ${d}.`,
    needPotion: 'No potions left.', usedPotion: 'Drank a potion (+health).',
    waitTurn: 'You wait.', onStairs: 'Stairs here — press down again to descend.',
    enemy: {
      rat: 'Cave Rat', bat: 'Gloom Bat', skel: 'Skeleton', orc: 'Orc Brute',
      wraith: 'Wraith', dragon: 'Elder Wyrm',
    },
    gear: {
      dagger: 'Dagger', sword: 'Iron Sword', axe: 'War Axe', flail: 'Spiked Flail', runeblade: 'Runeblade',
      cloth: 'Cloth Tunic', leather: 'Leather Armor', chain: 'Chainmail', plate: 'Plate Armor', aegis: 'Aegis Mail',
    },
    potionTag: 'Potions',
  },
  zh: {
    title: '地牢探险',
    subtitle: '回合制的黑暗爬行。每一步都是一回合。',
    play: '下潜', resume: '继续探险', howto: '方向键 / WASD 或滑动移动，撞向敌人即攻击。',
    depth: '深度', floor: '层', hp: '生命', lvl: '等级', gold: '金币', xp: '经验',
    weapon: '武器', armor: '护甲',
    paused: '已暂停', restart: '新探险', menu: '菜单', close: '继续',
    gameOver: '你死了', victory: '巨龙陨落',
    reachedDepth: d => `你倒在了第 ${d} 层。`,
    victoryMsg: '巨龙已被斩杀 —— 但深渊永无止境，继续下潜吧。',
    bestDepth: d => `最深记录：第 ${d} 层`,
    again: '再次探险', keepGoing: '继续下潜',
    foundWeapon: n => `获得 ${n}`, foundArmor: n => `获得 ${n}`,
    quaff: '喝下了一瓶药水', gotGold: g => `+${g} 金币`,
    levelUp: l => `升级！${l} 级`, descend: d => `你下潜到第 ${d} 层。`,
    hitFor: (n, d) => `你对 ${n} 造成 ${d} 伤害。`, slain: n => `${n} 被击杀！`,
    tookHit: (n, d) => `${n} 对你造成 ${d} 伤害。`,
    needPotion: '没有药水了。', usedPotion: '喝下药水（恢复生命）。',
    waitTurn: '你原地等待。', onStairs: '楼梯在此 —— 再次按下方向键下潜。',
    enemy: {
      rat: '洞穴鼠', bat: '幽暗蝙蝠', skel: '骷髅', orc: '兽人蛮兵',
      wraith: '怨灵', dragon: '远古巨龙',
    },
    gear: {
      dagger: '匕首', sword: '铁剑', axe: '战斧', flail: '钉头锤', runeblade: '符文之刃',
      cloth: '布衣', leather: '皮甲', chain: '锁子甲', plate: '板甲', aegis: '神盾甲',
    },
    potionTag: '药水',
  },
};

function t(key, ...args) {
  const v = TEXT[currentLang][key] ?? TEXT.en[key] ?? key;
  return typeof v === 'function' ? v(...args) : v;
}
function tEnemy(glyph) { return TEXT[currentLang].enemy[glyph] || glyph; }
function tGear(id) { return TEXT[currentLang].gear[id] || id; }

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

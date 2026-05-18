// Pixel Card Spire - localization
const LANG_KEY = 'pixel-spire-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'PIXEL CARD SPIRE',
    subtitle: 'Climb the spire. Build your deck. Survive.',
    play: 'NEW RUN',
    continueRun: 'CONTINUE',
    howto: 'Play cards with Energy each turn. Read enemy intents. Defeat The Warden.',
    floor: 'Floor',
    hp: 'HP', gold: 'Gold', energy: 'Energy',
    endTurn: 'End Turn', turn: 'Turn',
    deck: 'Deck', draw: 'Draw', discard: 'Discard',
    block: 'Block',
    chooseCard: 'Choose a card', skip: 'Skip',
    rewardTitle: 'SPOILS', cardReward: 'Add a card', goldReward: g => `+${g} gold`,
    relicReward: 'New relic!',
    proceed: 'Proceed', mapTitle: 'THE SPIRE',
    restTitle: 'CAMPFIRE', restHeal: 'Rest — heal 30%', restUpgrade: 'Forge — upgrade a card',
    shopTitle: 'SHOP', removeCard: cost => `Remove a card (${cost}g)`, buy: 'Buy', sold: 'SOLD',
    leave: 'Leave', notEnough: 'Not enough gold',
    victory: 'THE WARDEN FALLS', defeat: 'YOU DIED',
    victorySub: 'The spire is yours.', defeatSub: floor => `You fell on floor ${floor}.`,
    newRun: 'New Run', menu: 'Menu',
    upgrade: 'Upgrade', upgraded: 'Upgraded',
    yourDeck: 'Your Deck', relics: 'Relics', close: 'Close',
    intentAttack: 'Attacks', intentBlock: 'Defends', intentBuff: 'Buffs', intentDebuff: 'Weakens', intentUnknown: 'Unknown',
    str: 'Strength', vuln: 'Vulnerable', weak: 'Weak', poison: 'Poison',
    eliteWarn: 'ELITE', bossWarn: 'BOSS',
    confirmAbandon: 'Abandon this run?',
    cardUpgradeHint: 'Tap a card to upgrade it.',
    cardRemoveHint: 'Tap a card to remove it.',
    exhausted: 'No cards left to draw.',
    dmgWord: v => `Deal ${v} damage`,
    blockWord: v => `Gain ${v} Block`,
    drawWord: v => `Draw ${v} card${v > 1 ? 's' : ''}`,
    energyWord: v => `Gain ${v} Energy`,
    strWord: v => `Gain ${v} Strength`,
    vulnWord: v => `Apply ${v} Vulnerable`,
    weakWord: v => `Apply ${v} Weak`,
    poisonWord: v => `Apply ${v} Poison`,
    healWord: v => `Heal ${v} HP`,
    loseHpWord: v => `Lose ${v} HP`,
    toAll: ' to ALL', timesWord: n => ` ${n}x`,
    pwDemon: v => `At the start of each turn, gain ${v} Strength.`,
    pwBerserk: v => `At the start of each turn, gain ${v} Energy.`,
    pwJugg: v => `Whenever you gain Block, deal ${v} damage to a random enemy.`,
    pwMetal: v => `At the end of each turn, gain ${v} Block.`,
    spRampage: 'Deal damage. Permanently raises its own damage by 4 this combat.',
    spReaper: 'Heal HP equal to unblocked damage dealt.',
    spLimit: 'Double your Strength.',
  },
  zh: {
    title: '像素卡牌尖塔',
    subtitle: '攀登尖塔，构筑卡组，活下去。',
    play: '新征程',
    continueRun: '继续',
    howto: '每回合用能量打出卡牌，留意敌人意图，击败典狱长。',
    floor: '层',
    hp: '生命', gold: '金币', energy: '能量',
    endTurn: '结束回合', turn: '回合',
    deck: '卡组', draw: '抽牌堆', discard: '弃牌堆',
    block: '格挡',
    chooseCard: '选择一张卡', skip: '跳过',
    rewardTitle: '战利品', cardReward: '加入一张卡', goldReward: g => `+${g} 金币`,
    relicReward: '新遗物！',
    proceed: '继续前进', mapTitle: '尖塔',
    restTitle: '营火', restHeal: '休息 — 恢复30%', restUpgrade: '锻造 — 升级一张牌',
    shopTitle: '商店', removeCard: cost => `移除一张卡（${cost}金）`, buy: '购买', sold: '已售',
    leave: '离开', notEnough: '金币不足',
    victory: '典狱长已倒下', defeat: '你死了',
    victorySub: '尖塔已属于你。', defeatSub: floor => `你倒在了第 ${floor} 层。`,
    newRun: '新征程', menu: '主菜单',
    upgrade: '升级', upgraded: '已升级',
    yourDeck: '你的卡组', relics: '遗物', close: '关闭',
    intentAttack: '攻击', intentBlock: '防御', intentBuff: '强化', intentDebuff: '削弱', intentUnknown: '未知',
    str: '力量', vuln: '易伤', weak: '虚弱', poison: '中毒',
    eliteWarn: '精英', bossWarn: '首领',
    confirmAbandon: '放弃本次征程？',
    cardUpgradeHint: '点击一张牌进行升级。',
    cardRemoveHint: '点击一张牌进行移除。',
    exhausted: '没有可抽的牌了。',
    dmgWord: v => `造成 ${v} 点伤害`,
    blockWord: v => `获得 ${v} 点格挡`,
    drawWord: v => `抽 ${v} 张牌`,
    energyWord: v => `获得 ${v} 点能量`,
    strWord: v => `获得 ${v} 点力量`,
    vulnWord: v => `施加 ${v} 层易伤`,
    weakWord: v => `施加 ${v} 层虚弱`,
    poisonWord: v => `施加 ${v} 层中毒`,
    healWord: v => `恢复 ${v} 点生命`,
    loseHpWord: v => `失去 ${v} 点生命`,
    toAll: '（全体）', timesWord: n => `（${n}次）`,
    pwDemon: v => `每回合开始时获得 ${v} 点力量。`,
    pwBerserk: v => `每回合开始时获得 ${v} 点能量。`,
    pwJugg: v => `每当获得格挡时，对随机敌人造成 ${v} 点伤害。`,
    pwMetal: v => `每回合结束时获得 ${v} 点格挡。`,
    spRampage: '造成伤害。本场战斗每次打出后其伤害永久提升4点。',
    spReaper: '回复等同于造成的未格挡伤害的生命。',
    spLimit: '使你的力量翻倍。',
  },
};

function t(key, ...args) {
  const v = TEXT[currentLang][key] ?? TEXT.en[key] ?? key;
  return typeof v === 'function' ? v(...args) : v;
}
function L(pair) { return currentLang === 'zh' ? pair[1] : pair[0]; }

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

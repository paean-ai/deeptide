// Pixel Card Spire - game content: cards, enemies, relics, events

// ---- Cards -------------------------------------------------------------
// type: attack | skill | power
// effects: [{op, v}]  ops: dmg block draw energy str vuln weak poison heal loseHp
// flags: aoe (multi-target ops), hits (repeat dmg)
// u: sparse upgrade overrides (cost, effects, hits, power)
// special: id handled by the engine for non-standard behaviour
const CARD_DEFS = {
  // --- starter / common ---
  strike:   { name: ['Strike','重击'], type: 'attack', rarity: 'starter', cost: 1,
              effects: [{ op: 'dmg', v: 6 }], u: { effects: [{ op: 'dmg', v: 9 }] } },
  guard:    { name: ['Guard','格挡'], type: 'skill', rarity: 'starter', cost: 1,
              effects: [{ op: 'block', v: 5 }], u: { effects: [{ op: 'block', v: 8 }] } },
  bash:     { name: ['Bash','痛击'], type: 'attack', rarity: 'starter', cost: 2,
              effects: [{ op: 'dmg', v: 8 }, { op: 'vuln', v: 2 }],
              u: { effects: [{ op: 'dmg', v: 10 }, { op: 'vuln', v: 3 }] } },
  cleave:   { name: ['Cleave','横扫'], type: 'attack', rarity: 'common', cost: 1, aoe: true,
              effects: [{ op: 'dmg', v: 8 }], u: { effects: [{ op: 'dmg', v: 11 }] } },
  quickJab: { name: ['Quick Jab','迅刺'], type: 'attack', rarity: 'common', cost: 0,
              effects: [{ op: 'dmg', v: 4 }, { op: 'draw', v: 1 }],
              u: { effects: [{ op: 'dmg', v: 6 }, { op: 'draw', v: 1 }] } },
  ironWall: { name: ['Iron Wall','铁壁'], type: 'skill', rarity: 'common', cost: 1,
              effects: [{ op: 'block', v: 7 }], u: { effects: [{ op: 'block', v: 10 }] } },
  pommel:   { name: ['Pommel Strike','柄击'], type: 'attack', rarity: 'common', cost: 1,
              effects: [{ op: 'dmg', v: 7 }, { op: 'draw', v: 1 }],
              u: { effects: [{ op: 'dmg', v: 9 }, { op: 'draw', v: 2 }] } },
  twinSlice:{ name: ['Twin Slice','双斩'], type: 'attack', rarity: 'common', cost: 1, hits: 2,
              effects: [{ op: 'dmg', v: 4 }], u: { hits: 2, effects: [{ op: 'dmg', v: 5 }] } },
  // --- uncommon ---
  rampage:  { name: ['Rampage','暴怒'], type: 'attack', rarity: 'uncommon', cost: 1,
              effects: [{ op: 'dmg', v: 7 }], special: 'rampage',
              u: { effects: [{ op: 'dmg', v: 7 }], special: 'rampage' } },
  bloodlet: { name: ['Bloodletting','放血'], type: 'skill', rarity: 'uncommon', cost: 0,
              effects: [{ op: 'loseHp', v: 3 }, { op: 'energy', v: 2 }],
              u: { effects: [{ op: 'loseHp', v: 3 }, { op: 'energy', v: 3 }] } },
  riposte:  { name: ['Riposte','还击'], type: 'attack', rarity: 'uncommon', cost: 2,
              effects: [{ op: 'block', v: 6 }, { op: 'dmg', v: 6 }],
              u: { effects: [{ op: 'block', v: 9 }, { op: 'dmg', v: 9 }] } },
  inflame:  { name: ['Inflame','燃怒'], type: 'skill', rarity: 'uncommon', cost: 1,
              effects: [{ op: 'str', v: 2 }], u: { effects: [{ op: 'str', v: 3 }] } },
  shrugOff: { name: ['Shrug It Off','无视'], type: 'skill', rarity: 'uncommon', cost: 1,
              effects: [{ op: 'block', v: 8 }, { op: 'draw', v: 1 }],
              u: { effects: [{ op: 'block', v: 11 }, { op: 'draw', v: 1 }] } },
  whirl:    { name: ['Whirlwind','旋风斩'], type: 'attack', rarity: 'uncommon', cost: 2, aoe: true, hits: 2,
              effects: [{ op: 'dmg', v: 6 }], u: { hits: 3, effects: [{ op: 'dmg', v: 6 }] } },
  poisonBl: { name: ['Poison Blade','毒刃'], type: 'attack', rarity: 'uncommon', cost: 1,
              effects: [{ op: 'dmg', v: 5 }, { op: 'poison', v: 4 }],
              u: { effects: [{ op: 'dmg', v: 5 }, { op: 'poison', v: 6 }] } },
  secWind:  { name: ['Second Wind','回气'], type: 'skill', rarity: 'uncommon', cost: 1,
              effects: [{ op: 'heal', v: 6 }, { op: 'block', v: 5 }],
              u: { effects: [{ op: 'heal', v: 9 }, { op: 'block', v: 8 }] } },
  trance:   { name: ['Battle Trance','战意'], type: 'skill', rarity: 'uncommon', cost: 0,
              effects: [{ op: 'draw', v: 3 }], u: { effects: [{ op: 'draw', v: 4 }] } },
  intimid:  { name: ['Intimidate','威吓'], type: 'skill', rarity: 'uncommon', cost: 1, aoe: true,
              effects: [{ op: 'weak', v: 2 }], u: { effects: [{ op: 'weak', v: 3 }] } },
  // --- rare ---
  offering: { name: ['Offering','献祭'], type: 'skill', rarity: 'rare', cost: 0,
              effects: [{ op: 'loseHp', v: 6 }, { op: 'energy', v: 2 }, { op: 'draw', v: 3 }],
              u: { effects: [{ op: 'loseHp', v: 6 }, { op: 'energy', v: 2 }, { op: 'draw', v: 5 }] } },
  reaper:   { name: ['Reaper','收割'], type: 'attack', rarity: 'rare', cost: 2, aoe: true,
              effects: [{ op: 'dmg', v: 4 }], special: 'reaper',
              u: { effects: [{ op: 'dmg', v: 6 }], special: 'reaper' } },
  impervious:{ name: ['Impervious','金钟罩'], type: 'skill', rarity: 'rare', cost: 2,
              effects: [{ op: 'block', v: 30 }], u: { effects: [{ op: 'block', v: 40 }] } },
  limitBrk: { name: ['Limit Break','极限突破'], type: 'skill', rarity: 'rare', cost: 1,
              effects: [], special: 'limitBreak', u: { effects: [], special: 'limitBreak' } },
  demonForm:{ name: ['Demon Form','恶魔形态'], type: 'power', rarity: 'rare', cost: 3,
              effects: [], power: 'demon', powerV: 2, u: { powerV: 3, power: 'demon' } },
  berserk:  { name: ['Berserk','狂战'], type: 'power', rarity: 'rare', cost: 0,
              effects: [], power: 'berserk', powerV: 1, u: { powerV: 1, power: 'berserk' } },
  juggern:  { name: ['Juggernaut','势不可挡'], type: 'power', rarity: 'rare', cost: 2,
              effects: [], power: 'juggernaut', powerV: 5, u: { powerV: 8, power: 'juggernaut' } },
  metal:    { name: ['Metallicize','金属化'], type: 'power', rarity: 'uncommon', cost: 1,
              effects: [], power: 'metal', powerV: 3, u: { powerV: 4, power: 'metal' } },
  // --- expansion: common ---
  sidestep: { name: ['Sidestep','侧步'], type: 'skill', rarity: 'common', cost: 1,
              effects: [{ op: 'block', v: 5 }, { op: 'draw', v: 1 }],
              u: { effects: [{ op: 'block', v: 8 }, { op: 'draw', v: 1 }] } },
  hack:     { name: ['Hack','劈砍'], type: 'attack', rarity: 'common', cost: 1,
              effects: [{ op: 'dmg', v: 6 }, { op: 'weak', v: 1 }],
              u: { effects: [{ op: 'dmg', v: 8 }, { op: 'weak', v: 2 }] } },
  // --- expansion: uncommon ---
  frenzy:   { name: ['Frenzy','狂乱'], type: 'attack', rarity: 'uncommon', cost: 1, hits: 3,
              effects: [{ op: 'dmg', v: 3 }], u: { hits: 3, effects: [{ op: 'dmg', v: 4 }] } },
  toxicCloud:{ name: ['Toxic Cloud','毒云'], type: 'skill', rarity: 'uncommon', cost: 1, aoe: true,
              effects: [{ op: 'poison', v: 3 }], u: { effects: [{ op: 'poison', v: 5 }] } },
  fortify:  { name: ['Fortify','加固'], type: 'skill', rarity: 'uncommon', cost: 2,
              effects: [{ op: 'block', v: 14 }], u: { effects: [{ op: 'block', v: 20 }] } },
  reckless: { name: ['Reckless Strike','鲁莽一击'], type: 'attack', rarity: 'uncommon', cost: 1,
              effects: [{ op: 'dmg', v: 9 }, { op: 'loseHp', v: 2 }],
              u: { effects: [{ op: 'dmg', v: 13 }, { op: 'loseHp', v: 2 }] } },
  // --- expansion: rare ---
  execute:  { name: ['Execute','处决'], type: 'attack', rarity: 'rare', cost: 2,
              effects: [{ op: 'dmg', v: 20 }], u: { effects: [{ op: 'dmg', v: 28 }] } },
  bulwark:  { name: ['Bulwark','壁垒'], type: 'skill', rarity: 'rare', cost: 1,
              effects: [{ op: 'block', v: 10 }, { op: 'str', v: 2 }],
              u: { effects: [{ op: 'block', v: 14 }, { op: 'str', v: 3 }] } },
  venomBurst:{ name: ['Venom Burst','剧毒爆发'], type: 'attack', rarity: 'rare', cost: 1, aoe: true,
              effects: [{ op: 'dmg', v: 6 }, { op: 'poison', v: 5 }],
              u: { effects: [{ op: 'dmg', v: 8 }, { op: 'poison', v: 7 }] } },
  adrenaline:{ name: ['Adrenaline','肾上腺素'], type: 'skill', rarity: 'rare', cost: 0,
              effects: [{ op: 'energy', v: 2 }, { op: 'draw', v: 2 }],
              u: { effects: [{ op: 'energy', v: 3 }, { op: 'draw', v: 2 }] } },
  // --- expansion 2: common ---
  heavyGuard:{ name: ['Heavy Guard','重甲'], type: 'skill', rarity: 'common', cost: 2,
              effects: [{ op: 'block', v: 13 }], u: { effects: [{ op: 'block', v: 18 }] } },
  gash:     { name: ['Gash','割裂'], type: 'attack', rarity: 'common', cost: 1,
              effects: [{ op: 'dmg', v: 6 }, { op: 'vuln', v: 1 }],
              u: { effects: [{ op: 'dmg', v: 8 }, { op: 'vuln', v: 2 }] } },
  // --- expansion 2: uncommon ---
  warcry:   { name: ['War Cry','战吼'], type: 'skill', rarity: 'uncommon', cost: 0,
              effects: [{ op: 'str', v: 1 }, { op: 'draw', v: 1 }],
              u: { effects: [{ op: 'str', v: 1 }, { op: 'draw', v: 2 }] } },
  siphon:   { name: ['Siphon Strike','汲取'], type: 'attack', rarity: 'uncommon', cost: 1,
              effects: [{ op: 'dmg', v: 6 }, { op: 'heal', v: 3 }],
              u: { effects: [{ op: 'dmg', v: 8 }, { op: 'heal', v: 5 }] } },
  crescent: { name: ['Crescent Sweep','弦月斩'], type: 'attack', rarity: 'uncommon', cost: 2, aoe: true,
              effects: [{ op: 'dmg', v: 9 }, { op: 'weak', v: 1 }],
              u: { effects: [{ op: 'dmg', v: 12 }, { op: 'weak', v: 2 }] } },
  // --- expansion 2: rare ---
  cataclysm:{ name: ['Cataclysm','天灾'], type: 'attack', rarity: 'rare', cost: 3, aoe: true,
              effects: [{ op: 'dmg', v: 14 }], u: { effects: [{ op: 'dmg', v: 20 }] } },
  // --- expansion 3: common ---
  slash:    { name: ['Slash','斩击'], type: 'attack', rarity: 'common', cost: 1,
              effects: [{ op: 'dmg', v: 8 }], u: { effects: [{ op: 'dmg', v: 11 }] } },
  parry:    { name: ['Parry','招架'], type: 'skill', rarity: 'common', cost: 1,
              effects: [{ op: 'block', v: 5 }, { op: 'weak', v: 1 }],
              u: { effects: [{ op: 'block', v: 7 }, { op: 'weak', v: 1 }] } },
  // --- expansion 3: uncommon ---
  bloodrage:{ name: ['Bloodrage','血怒'], type: 'attack', rarity: 'uncommon', cost: 1,
              effects: [{ op: 'dmg', v: 7 }, { op: 'str', v: 1 }],
              u: { effects: [{ op: 'dmg', v: 9 }, { op: 'str', v: 2 }] } },
  barricade:{ name: ['Barricade','壁障'], type: 'skill', rarity: 'uncommon', cost: 2,
              effects: [{ op: 'block', v: 10 }, { op: 'draw', v: 2 }],
              u: { effects: [{ op: 'block', v: 14 }, { op: 'draw', v: 2 }] } },
  // --- expansion 3: rare ---
  rupture:  { name: ['Rupture','破裂'], type: 'attack', rarity: 'rare', cost: 1,
              effects: [{ op: 'dmg', v: 9 }, { op: 'vuln', v: 2 }, { op: 'weak', v: 2 }],
              u: { effects: [{ op: 'dmg', v: 13 }, { op: 'vuln', v: 3 }, { op: 'weak', v: 3 }] } },
  bloodfeast:{ name: ['Blood Feast','血宴'], type: 'attack', rarity: 'rare', cost: 2,
              effects: [{ op: 'dmg', v: 16 }, { op: 'heal', v: 6 }],
              u: { effects: [{ op: 'dmg', v: 22 }, { op: 'heal', v: 9 }] } },
};

const STARTER_DECK = ['strike','strike','strike','strike','strike','guard','guard','guard','guard','bash'];

const COMMON_POOL = ['cleave','quickJab','ironWall','pommel','twinSlice','sidestep','hack','heavyGuard','gash','slash','parry'];
const UNCOMMON_POOL = ['rampage','bloodlet','riposte','inflame','shrugOff','whirl','poisonBl','secWind','trance','intimid','metal','frenzy','toxicCloud','fortify','reckless','warcry','siphon','crescent','bloodrage','barricade'];
const RARE_POOL = ['offering','reaper','impervious','limitBrk','demonForm','berserk','juggern','execute','bulwark','venomBurst','adrenaline','cataclysm','rupture','bloodfeast'];

function rollCardReward(rng, count) {
  const out = [];
  while (out.length < count) {
    const r = rng();
    let pool, rar;
    if (r < 0.62) { pool = COMMON_POOL; rar = 'common'; }
    else if (r < 0.92) { pool = UNCOMMON_POOL; rar = 'uncommon'; }
    else { pool = RARE_POOL; rar = 'rare'; }
    const id = pool[Math.floor(rng() * pool.length)];
    if (!out.includes(id)) out.push(id);
  }
  return out;
}

// ---- Enemies -----------------------------------------------------------
// move types: attack(v,hits) block(v) buff(str) debuff(weak|vuln to player) atkbuff atkdebuff
const ENEMY_DEFS = {
  slime:   { name: ['Slime','史莱姆'], hp: [30,34], sprite: 'slime', color: '#6fae3e',
             pattern: ['attack:7','block:5','attack:9'] },
  bat:     { name: ['Bat','蝙蝠'], hp: [18,22], sprite: 'bat', color: '#9a7ad8',
             pattern: ['multi:4:2','attack:6'] },
  cultist: { name: ['Cultist','邪教徒'], hp: [24,28], sprite: 'humanoid', color: '#c0392b',
             pattern: ['buff:3','grow:6'] },
  bandit:  { name: ['Bandit','强盗'], hp: [30,34], sprite: 'humanoid', color: '#caa14a',
             pattern: ['attack:9','atkdebuff:5:weak:2'] },
  spiker:  { name: ['Spike Beast','尖刺兽'], hp: [27,31], sprite: 'beast', color: '#d97a3a',
             pattern: ['attack:6','attack:8','block:8'], thorns: 3 },
  shield:  { name: ['Shieldbearer','持盾兵'], hp: [36,42], sprite: 'knight', color: '#7f93b8',
             pattern: ['block:12','attack:6','attack:9'] },
  witch:   { name: ['Hex Witch','巫女'], hp: [22,26], sprite: 'humanoid', color: '#7d4fbe',
             pattern: ['debuff:vuln:2','attack:5','debuff:weak:2'] },
  wolf:    { name: ['Dire Wolf','恶狼'], hp: [21,25], sprite: 'beast', color: '#8a8f9a',
             pattern: ['attack:5','attbuff:8:2','attack:7'] },
  // elites
  golem:   { name: ['Stone Golem','石巨人'], hp: [78,86], sprite: 'golem', color: '#8a93a8', elite: true,
             pattern: ['attbuff:14:3','block:14','attack:18','multi:6:3'] },
  twins:   { name: ['Twin Blades','双刃'], hp: [54,60], sprite: 'knight', color: '#c0556b', elite: true,
             pattern: ['multi:5:3','buff:4','attack:16','block:10'] },
  // boss
  warden:  { name: ['The Warden','典狱长'], hp: [172,172], sprite: 'boss', color: '#b03a4a', boss: true,
             pattern: ['attbuff:20:3','block:18','multi:7:3','debuff:vuln:3','attack:30'] },
};

// ---- Relics ------------------------------------------------------------
const RELIC_DEFS = {
  anchor:   { name: ['Bronze Anchor','青铜锚'], desc: ['Start each combat with 8 Block.','每场战斗开始获得8格挡。'], hook: 'combatStart' },
  warDrum:  { name: ['War Drum','战鼓'], desc: ['Start each combat with 1 Strength.','每场战斗开始获得1力量。'], hook: 'combatStart' },
  whetstone:{ name: ['Whetstone','磨刀石'], desc: ['Strike cards deal 2 extra damage.','重击牌额外造成2点伤害。'], hook: 'passive' },
  medkit:   { name: ['Medkit','医疗包'], desc: ['Heal 7 HP after each combat.','每场战斗后恢复7点生命。'], hook: 'combatEnd' },
  thickHide:{ name: ['Thick Hide','厚皮'], desc: ['Raises Max HP by 12.','最大生命提升12。'], hook: 'pickup' },
  oldTome:  { name: ['Old Tome','古籍'], desc: ['Draw 1 extra card on turn 1.','第一回合多抽1张牌。'], hook: 'combatStart' },
  crackedCore:{ name: ['Cracked Core','破裂核心'], desc: ['Gain 1 Energy on turn 1.','第一回合获得1能量。'], hook: 'combatStart' },
  gingerRoot:{ name: ['Ginger Root','姜根'], desc: ['You can no longer become Weak.','你不再会被虚弱。'], hook: 'passive' },
  vampFang: { name: ['Vampire Fang','吸血獠牙'], desc: ['Heal 3 HP whenever an enemy dies.','敌人死亡时恢复3点生命。'], hook: 'kill' },
  handCannon:{ name: ['Hand Cannon','手炮'], desc: ['Deal 8 damage to ALL enemies at combat start.','战斗开始对所有敌人造成8点伤害。'], hook: 'combatStart' },
  luckyCoin:{ name: ['Lucky Coin','幸运币'], desc: ['Gain 25 extra gold from combats.','战斗额外获得25金币。'], hook: 'reward' },
  energyCore:{ name: ['Energy Core','能量核心'], desc: ['Gain 1 extra Energy each turn.','每回合额外获得1能量。'], hook: 'passive', boss: true },
  ironCharm:{ name: ['Iron Charm','铁符'], desc: ['First time HP drops below half each combat, gain 3 Strength.','每场战斗首次生命低于一半时获得3力量。'], hook: 'bloodied' },
  cloverLeaf:{ name: ['Lucky Clover','幸运草'], desc: ['Card rewards offer 4 choices.','卡牌奖励提供4个选择。'], hook: 'passive' },
};
const RELIC_POOL = ['anchor','warDrum','whetstone','medkit','thickHide','oldTome','crackedCore','gingerRoot','vampFang','handCannon','luckyCoin','ironCharm','cloverLeaf'];

// ---- Events ------------------------------------------------------------
const EVENT_DEFS = [
  {
    id: 'shrine', title: ['Forgotten Shrine','被遗忘的神龛'],
    text: ['A cracked altar hums with old power. An offering bowl waits.',
           '裂开的祭坛回响着古老的力量，献祭碗静静等待。'],
    choices: [
      { label: ['Offer 8 HP — gain a relic','献祭8点生命 — 获得遗物'], act: 'shrine_relic' },
      { label: ['Pray — heal 14 HP','祈祷 — 恢复14点生命'], act: 'shrine_heal' },
      { label: ['Leave','离开'], act: 'nothing' },
    ],
  },
  {
    id: 'merchant', title: ['Wandering Merchant','流浪商人'],
    text: ['A hooded trader grins. "A gamble, friend? Coin for power."',
           '兜帽商人咧嘴一笑：“赌一把吗朋友？金币换力量。”'],
    choices: [
      { label: ['Pay 45 gold — upgrade a card','支付45金币 — 升级一张牌'], act: 'merchant_upgrade' },
      { label: ['Pay 30 gold — remove a card','支付30金币 — 移除一张牌'], act: 'merchant_remove' },
      { label: ['Decline','拒绝'], act: 'nothing' },
    ],
  },
  {
    id: 'training', title: ['Old Training Hall','古老训练场'],
    text: ['Dusty dummies and a worn whetstone. You could drill, or rest.',
           '布满灰尘的木桩和磨损的磨石。你可以训练，或休息。'],
    choices: [
      { label: ['Drill hard — gain a card, lose 6 HP','刻苦训练 — 获得卡牌，失去6生命'], act: 'training_card' },
      { label: ['Light practice — heal 10 HP','轻度练习 — 恢复10生命'], act: 'shrine_heal2' },
      { label: ['Leave','离开'], act: 'nothing' },
    ],
  },
];

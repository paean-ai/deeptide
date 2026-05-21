// Pixel Elemancer - English / Chinese strings.

const STRINGS = {
  en: {
    title: 'PIXEL ELEMANCER',
    subtitle: 'Four forms. One foe.',
    start: 'START',
    foeSelect: 'CHOOSE A FOE',
    locked: 'LOCKED',
    cleared: 'CLEARED',
    back: 'MENU',
    strike: 'STRIKE',
    guard: 'GUARD',
    special: 'SPECIAL',
    chooseForm: 'SHIFT TO A FORM',
    win: 'FOE FELLED',
    lose: 'OVERWHELMED',
    retry: 'RETRY',
    next: 'NEXT FOE',
    used: 'USED',
    charged: 'CHARGING',
    empowered: 'EMPOWERED',
    threat: 'NEXT HIT',
    rules1: 'fire > grass > storm > water > fire — each beats the next.',
    rules2: 'Strike in a super form. Guard a charged hit to halve it and empower your next blow.',
    rules3: 'A voluntary shift costs the turn. Fell the foe before all four forms fall.',
  },
  zh: {
    title: '像素元能师',
    subtitle: '四相之力，元素相克',
    start: '开始',
    foeSelect: '选择对手',
    locked: '未解锁',
    cleared: '已通关',
    back: '菜单',
    strike: '攻击',
    guard: '格挡',
    special: '绝技',
    chooseForm: '换相',
    win: '击败对手',
    lose: '力竭',
    retry: '重试',
    next: '下一个',
    used: '已用',
    charged: '蓄力中',
    empowered: '蓄势',
    threat: '下次受击',
    rules1: '火 > 草 > 雷 > 水 > 火，依次克制。',
    rules2: '用克制的相位攻击。格挡蓄力攻击可减半伤害并强化下一击。',
    rules3: '主动换相会消耗回合。在四相耗尽前击败对手。',
  },
};

// special display names + one-line descriptions, keyed by FORMS special key
const SPECIAL_NAME = {
  pyre:    ['Pyre', '烈焰'],
  bulwark: ['Bulwark', '壁垒'],
  tempo:   ['Tempo', '疾节'],
  mend:    ['Mend', '润复'],
};
const SPECIAL_DESC = {
  pyre:    ['Heavy blow, slight recoil', '强力一击，略有反伤'],
  bulwark: ['Heal, guard and empower', '回复并格挡、蓄势'],
  tempo:   ['Strike twice in a turn', '一回合内连击两次'],
  mend:    ['Heal every living form', '回复所有存活相位'],
};

let lang = 'en';
function loadLang() {
  try {
    const v = localStorage.getItem('pixel-elemancer:lang');
    if (v === 'en' || v === 'zh') lang = v;
  } catch (e) { /* storage unavailable */ }
}
function saveLang() {
  try { localStorage.setItem('pixel-elemancer:lang', lang); } catch (e) { /* ignore */ }
}
function t(key) { return STRINGS[lang][key]; }
function L(pair) { return pair[lang === 'zh' ? 1 : 0]; }

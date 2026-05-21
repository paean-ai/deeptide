// English / 中文 strings for Pixel Leap.

const STRINGS = {
  en: {
    title: 'PIXEL LEAP',
    subtitle: 'Run, jump and dash to the door',
    start: 'Start',
    levelSelect: 'Choose room',
    cleared: 'Cleared',
    locked: 'Locked',
    play: 'Play',
    backToMenu: 'Menu',
    win: 'Room cleared!',
    next: 'Next room',
    retry: 'Retry',
    deaths: 'Deaths',
    rulesTxt1: '◀ ▶ to move, ▲ to jump (hold for height).',
    rulesTxt2: '» air-dashes once - refreshed when you land.',
    rulesTxt3: 'Reach the green door; spikes and pits respawn you.',
  },
  zh: {
    title: '像素跃迁',
    subtitle: '奔跑、跳跃、冲刺，抵达门口',
    start: '开始',
    levelSelect: '选择房间',
    cleared: '已通关',
    locked: '未解锁',
    play: '游玩',
    backToMenu: '菜单',
    win: '过关！',
    next: '下一间',
    retry: '重试',
    deaths: '死亡',
    rulesTxt1: '◀ ▶ 移动，▲ 跳跃（按住跳更高）。',
    rulesTxt2: '» 空中冲刺一次，落地后恢复。',
    rulesTxt3: '抵达绿色门；碰到尖刺或坠落会重生。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-leap:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-leap:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }

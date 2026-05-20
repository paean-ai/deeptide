// English / 中文 strings for Pixel Pool.

const STRINGS = {
  en: {
    title: 'PIXEL POOL',
    subtitle: 'Drag away from the cue · release to shoot · pocket every ball',
    start: 'Start',
    levelSelect: 'Choose rack',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Rack clear!',
    lose: 'Out of strokes.',
    next: 'Next rack',
    retry: 'Retry',
    score: 'Score',
    strokes: 'Strokes',
    fouls: 'Fouls',
    aim: 'Drag from the cue ball, release to shoot',
    rulesTxt1: 'Drag AWAY from the cue (white) ball to aim and set power.',
    rulesTxt2: 'Pocket every colored ball before strokes run out.',
    rulesTxt3: 'Scratch (cue in pocket) = +1 foul, cue respawns at break.',
  },
  zh: {
    title: '像素台球',
    subtitle: '从白球反向拖动·松手出杆·全部入袋',
    start: '开始',
    levelSelect: '选择球架',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '清台！',
    lose: '杆数用尽。',
    next: '下一架',
    retry: '再来',
    score: '分数',
    strokes: '杆数',
    fouls: '犯规',
    aim: '从白球拖动，松手出杆',
    rulesTxt1: '从白球反方向拖动可瞄准并设定力度。',
    rulesTxt2: '在杆数用尽前把所有彩球击入袋中。',
    rulesTxt3: '白球入袋为犯规：+1 犯规，白球回到出杆点。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-pool:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-pool:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }

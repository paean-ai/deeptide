// Pixel Arcade - localization
const LANG_KEY = 'pixel-arcade-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'PIXEL ARCADE',
    subtitle: 'Five quick games. One pixel cabinet.',
    pick: 'Pick a game',
    howto: 'Each game is a 60-second skill test. Beat your best and chase gold medals.',
    play: 'PLAY', back: 'Back', retry: 'Retry', hub: 'Arcade',
    best: 'Best', score: 'Score', lives: 'Lives', tapStart: 'Tap to start',
    gameOver: 'GAME OVER', newBest: 'NEW BEST!',
    medals: 'Medals', medalNone: '—',
    paused: 'PAUSED', resume: 'Resume',
    games: {
      flap:   ['Sky Flap', '冲天小鸟'],
      catch:  ['Fruit Catch', '接水果'],
      reflex: ['Reflex Tap', '反应点击'],
      stack:  ['Tower Stack', '叠叠高'],
      dash:   ['Pixel Dash', '像素冲刺'],
    },
    instr: {
      flap:   'Tap to flap. Fly through the gaps.',
      catch:  'Drag to move the basket. Catch fruit, dodge bombs.',
      reflex: 'Tap the targets before they vanish.',
      stack:  'Tap to drop each block. Stack them high.',
      dash:   'Tap to jump. Leap over every obstacle.',
    },
    unit: {
      flap: 'gaps', catch: 'fruit', reflex: 'hits', stack: 'blocks', dash: 'm',
    },
  },
  zh: {
    title: '像素街机',
    subtitle: '五个小游戏，一台像素机厅。',
    pick: '选择一个游戏',
    howto: '每个游戏都是技巧考验，刷新纪录，争夺金牌。',
    play: '开始', back: '返回', retry: '重试', hub: '街机厅',
    best: '最佳', score: '得分', lives: '生命', tapStart: '点击开始',
    gameOver: '游戏结束', newBest: '新纪录！',
    medals: '奖牌', medalNone: '—',
    paused: '已暂停', resume: '继续',
    games: {
      flap:   ['Sky Flap', '冲天小鸟'],
      catch:  ['Fruit Catch', '接水果'],
      reflex: ['Reflex Tap', '反应点击'],
      stack:  ['Tower Stack', '叠叠高'],
      dash:   ['Pixel Dash', '像素冲刺'],
    },
    instr: {
      flap:   '点击让小鸟扇翅，穿过缝隙。',
      catch:  '拖动移动篮子，接住水果，躲开炸弹。',
      reflex: '在目标消失前点击它们。',
      stack:  '点击放下方块，把塔叠得更高。',
      dash:   '点击跳跃，越过所有障碍。',
    },
    unit: {
      flap: '个', catch: '个', reflex: '次', stack: '层', dash: '米',
    },
  },
};

function t(key) {
  return TEXT[currentLang][key] ?? TEXT.en[key] ?? key;
}
function gameName(id) { return TEXT[currentLang].games[id][currentLang === 'zh' ? 1 : 0]; }
function gameInstr(id) { return TEXT[currentLang].instr[id]; }
function gameUnit(id) { return TEXT[currentLang].unit[id]; }

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

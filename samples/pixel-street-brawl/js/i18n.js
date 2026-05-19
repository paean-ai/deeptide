// Pixel Street Brawl - localization (English / 中文)
const LANG_KEY = 'pixel-street-brawl-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'STREET BRAWL',
    subtitle: 'Punch, kick, and combo your way through the horde.',
    play: 'FIGHT', howto: 'Move with arrows / A·D. J punch, K kick, L jump. Or use the buttons.',
    wave: 'Wave', score: 'Score', best: 'Best',
    punch: 'PUNCH', kick: 'KICK', jump: 'JUMP',
    paused: 'PAUSED', resume: 'Resume', restart: 'Restart', menu: 'Menu',
    gameOver: 'KNOCKED OUT', finalScore: s => `Score: ${s}`,
    reachedWave: w => `Reached wave ${w}`, bestScore: s => `Best: ${s}`,
    again: 'BRAWL AGAIN', waveBanner: w => `WAVE ${w}`, bossBanner: 'BOSS FIGHT',
    cleared: 'WAVE CLEARED', combo: 'COMBO',
  },
  zh: {
    title: '街头格斗',
    subtitle: '出拳、踢腿、连段,杀穿整条街。',
    play: '开打', howto: '方向键 / A·D 移动。J 出拳,K 踢腿,L 跳跃。也可用按钮。',
    wave: '波次', score: '分数', best: '最高',
    punch: '拳', kick: '踢', jump: '跳',
    paused: '已暂停', resume: '继续', restart: '重新开始', menu: '菜单',
    gameOver: '被击倒', finalScore: s => `分数：${s}`,
    reachedWave: w => `到达第 ${w} 波`, bestScore: s => `最高：${s}`,
    again: '再战一场', waveBanner: w => `第 ${w} 波`, bossBanner: '首领战',
    cleared: '波次清空', combo: '连击',
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

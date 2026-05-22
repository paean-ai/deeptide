window.PixelArt = (() => {
  function block(ctx,x,y,w,h,c){ ctx.fillStyle=c; ctx.fillRect(Math.round(x),Math.round(y),Math.round(w),Math.round(h)); }
  function sprite(ctx,x,y,s,type,theme,level=1){
    const a=theme.accent,b=theme.accent2,d=theme.danger,soil=theme.soil,water=theme.water;
    block(ctx,x,y+s*3,s*10,s*8,'#090b10');
    if(type==='tree'){ block(ctx,x+s*4,y+s*5,s*3,s*7,soil); block(ctx,x+s*1,y+s*1,s*9,s*6,b); block(ctx,x+s*3,y,s*5,s*4,a); }
    else if(type==='water'){ block(ctx,x+s,y+s*5,s*10,s*5,water); block(ctx,x+s*2,y+s*3,s*7,s*2,b); block(ctx,x+s*6,y+s,s*3,s*2,'#d7fbff'); }
    else if(type==='fire'){ block(ctx,x+s*3,y+s*5,s*6,s*6,d); block(ctx,x+s*5,y+s*2,s*3,s*7,a); block(ctx,x+s*6,y+s*5,s*2,s*4,'#fff0a0'); }
    else if(type==='gear'){ block(ctx,x+s*2,y+s*3,s*8,s*8,a); block(ctx,x+s*4,y+s*5,s*4,s*4,'#141820'); block(ctx,x+s*5,y+s,s*2,s*12,b); block(ctx,x,y+s*6,s*12,s*2,b); }
    else if(type==='shop'){ block(ctx,x+s*1,y+s*5,s*10,s*7,soil); block(ctx,x,y+s*3,s*12,s*3,a); block(ctx,x+s*2,y+s*7,s*3,s*5,b); block(ctx,x+s*7,y+s*7,s*3,s*5,'#111820'); }
    else if(type==='tower'||type==='target'){ block(ctx,x+s*3,y+s*2,s*6,s*10,soil); block(ctx,x+s*2,y,s*8,s*3,a); block(ctx,x+s*4,y+s*5,s*4,s*3,b); if(type==='target'){ block(ctx,x+s*5,y+s*1,s*2,s*10,d); block(ctx,x+s*2,y+s*5,s*8,s*2,d); } }
    else if(type==='shield'){ block(ctx,x+s*2,y+s*2,s*8,s*4,b); block(ctx,x+s*3,y+s*6,s*6,s*5,a); block(ctx,x+s*5,y+s*7,s*2,s*2,'#111820'); }
    else if(type==='chip'){ block(ctx,x+s*2,y+s*3,s*8,s*7,b); block(ctx,x+s*4,y+s*5,s*4,s*3,'#111820'); for(let i=0;i<4;i++) block(ctx,x+s*(1+i*3),y+s*1,s,s*2,a); }
    else if(type==='crystal'||type==='egg'){ block(ctx,x+s*5,y,s*3,s*2,a); block(ctx,x+s*3,y+s*2,s*7,s*7,b); block(ctx,x+s*4,y+s*9,s*5,s*3,d); if(type==='egg') block(ctx,x+s*5,y+s*4,s*2,s*2,'#fff'); }
    else if(type==='pot'||type==='table'){ block(ctx,x+s*2,y+s*6,s*8,s*5,soil); block(ctx,x+s*3,y+s*4,s*6,s*2,a); if(type==='pot'){ block(ctx,x+s*4,y+s*2,s*4,s*3,d); } }
    else if(type==='crate'||type==='rock'){ block(ctx,x+s*2,y+s*4,s*8,s*7, type==='rock'?'#66717d':soil); block(ctx,x+s*3,y+s*5,s*2,s*2,a); block(ctx,x+s*7,y+s*7,s*2,s*2,b); }
    else if(type==='wheel'){ for(let i=0;i<8;i++){ const ang=i*Math.PI/4; block(ctx,x+s*6+Math.cos(ang)*s*4,y+s*6+Math.sin(ang)*s*4,s,s,a); } block(ctx,x+s*5,y+s*5,s*3,s*3,b); block(ctx,x+s*4,y+s*9,s*4,s*3,soil); }
    else if(type==='track'||type==='field'){ block(ctx,x+s,y+s*4,s*10,s*5,type==='field'?b:soil); for(let i=0;i<5;i++) block(ctx,x+s*(1+i*2),y+s*3,s,s*7,a); }
    else { block(ctx,x+s*2,y+s*3,s*8,s*8,soil); block(ctx,x+s*3,y+s,s*6,s*4,a); block(ctx,x+s*5,y+s*6,s*3,s*5,b); }
    if(level>1){ for(let i=0;i<level-1;i++) block(ctx,x+s*(1+i*2),y+s*12,s,s,a); }
  }
  return { block, sprite };
})();

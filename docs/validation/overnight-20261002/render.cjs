const { chromium } = require('C:/Users/puckh/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright');
const fs=require('fs'), path=require('path');
const dir=__dirname, fixtures=JSON.parse(fs.readFileSync(path.join(dir,'ui-fixture.json'),'utf8'));
(async()=>{
 const browser=await chromium.launch({headless:true,channel:"msedge"});
 const page=await browser.newPage({viewport:{width:1200,height:800}});
 const errors=[];page.on('pageerror',e=>errors.push(String(e)));
 await page.addInitScript(data=>{window.__TAURI_INTERNALS__={invoke:async(cmd,args)=>{
   if(cmd in data)return structuredClone(data[cmd]);
   if(cmd==='baseline_inventory')return {targets:[]};
   if(cmd==='project_set_view')return null;
   throw new Error('QA fixture has no resource for '+cmd);
 },transformCallback:()=>0,convertFileSrc:()=>''};},fixtures);
 await page.goto('http://127.0.0.1:1420/');
 await page.getByRole('heading',{name:'Project',exact:true}).waitFor();
 await page.screenshot({path:path.join(dir,'startup-1200.png')});
 await page.getByLabel('Existing project folder').fill('QA fixture (read-only browser responses)');
 await page.getByRole('button',{name:'Open',exact:true}).click();
 await page.getByRole('button',{name:'Continue to Studio',exact:true}).click();
 await page.getByLabel('Search teams').waitFor();
 await page.getByText('Anaheim Ducks®',{exact:true}).first().waitFor();
 await page.screenshot({path:path.join(dir,'teams-1200.png')});
 await page.getByText('Anaheim Ducks®',{exact:true}).first().click();
 await page.getByRole('heading',{name:'Anaheim Ducks®',exact:true}).waitFor();
 await page.screenshot({path:path.join(dir,'team-detail-1200.png')});
 await page.getByRole('link',{name:'Leagues',exact:true}).click();
 await page.getByLabel('Search leagues').waitFor();
 await page.screenshot({path:path.join(dir,'leagues-1200.png')});
 const first=fixtures.list_leagues.leagues.find(l=>l.code===0);
 await page.getByText(first.display,{exact:true}).first().click();
 await page.getByRole('heading',{name:first.display,exact:true}).waitFor();
 await page.screenshot({path:path.join(dir,'league-detail-1200.png')});
 await page.setViewportSize({width:900,height:650});
 await page.screenshot({path:path.join(dir,'league-detail-900.png')});
 const inspected=[];
 for(const route of ['database/players','database/arenas','database/game-settings','database/transactions','database/pending','historical/players','legends','shop','assets/players','assets/export','assets/appearance','assets/outfits','assets/files','settings/general','settings/paths','settings/dashboard','settings/about']) {
  await page.evaluate(url=>{history.pushState({},'',url);dispatchEvent(new PopStateEvent('popstate'));}, '/app/'+route);
  await page.waitForTimeout(700);
  inspected.push({route,overflow:await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth)});
  await page.screenshot({path:path.join(dir,route.replaceAll('/','-')+'-900.png')});
 }
 fs.writeFileSync(path.join(dir,'page-inventory.json'),JSON.stringify(inspected,null,2));
 fs.writeFileSync(path.join(dir,'browser-results.json'),JSON.stringify({errors,url:page.url(),overflow:await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth)},null,2));
 await browser.close();
})().catch(e=>{console.error(e);process.exit(1)});


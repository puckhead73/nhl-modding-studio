// Real browser layout regression; commands return owned synthetic discovery data.
const {chromium}=require('C:/Users/puckh/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright');
const fs=require('fs');
(async()=>{
 const browser=await chromium.launch({channel:'msedge',headless:true});
 try {
 const page=await browser.newPage({viewport:{width:1200,height:800}});
 const discovery=require('./ui-fixture.json').project_open.discovery;
 discovery.game_candidates=Array.from({length:15},(_,i)=>({path:'D:/QA/Games/'+i,title:'Synthetic NHL installation '+i,title_id:'CUSA'+i,app_version:'01.00',content_id:null,matches:i===14,reason:null}));
 await page.addInitScript(report=>{window.__TAURI_INTERNALS__={invoke:async(cmd)=>{
 if(cmd==='baseline_inventory')return {targets:[{game_id:'nhl21',platform:'ps4',title:'NHL 21',status:'partial',components:[],gaps:[{component:'dictionary',reason:'Synthetic missing optional dictionary'}]}]};
 if(cmd==='detect_install')return report;
 if(cmd==='ps4_save_root_suggestion')return {path:'D:/QA/Saves',exists:true};
 throw Error('Unprovided fixture command: '+cmd);
 },transformCallback:()=>0,convertFileSrc:()=>''};},discovery);
 await page.goto('http://127.0.0.1:1420');
 await page.getByLabel('Project folder',{exact:true}).fill('D:/QA/Project');
 await page.getByText('Start fresh',{exact:true}).click();
 await page.getByText('NHL 21',{exact:true}).click();
 await page.getByLabel('Game installation folder',{exact:true}).fill('D:/QA/Games');
 await page.getByText('Synthetic NHL installation 14',{exact:false}).waitFor();
 const results=[];
 for(const size of [{width:1200,height:800},{width:900,height:600},{width:640,height:480}]) {
 await page.setViewportSize(size);
 const panel=page.locator('.fixed.inset-0').first().locator(':scope > div').first();
 await panel.evaluate(el=>{el.scrollTop=0});
 const bounds=await panel.evaluate(el=>{const b=el.getBoundingClientRect();return {top:b.top,bottom:b.bottom,height:innerHeight,scrollHeight:el.scrollHeight,clientHeight:el.clientHeight,overflow:getComputedStyle(el).overflowY}});
 if(bounds.top<0||bounds.bottom>size.height||bounds.scrollHeight<=bounds.clientHeight||!['auto','scroll'].includes(bounds.overflow))throw Error('Unreachable panel: '+JSON.stringify(bounds));
 await page.getByRole('heading',{name:'Project, game and saves'}).scrollIntoViewIfNeeded();
 await page.screenshot({path:__dirname+'/scroll-top-'+size.width+'.png'});
 await page.getByRole('button',{name:'Next',exact:true}).scrollIntoViewIfNeeded();
 const next=await page.getByRole('button',{name:'Next',exact:true}).boundingBox();
 if(next.y<0||next.y+next.height>size.height)throw Error('Next remains clipped');
 await page.screenshot({path:__dirname+'/scroll-bottom-'+size.width+'.png'});
 results.push({size,bounds,next});
 }
 fs.writeFileSync(__dirname+'/scroll-results.json',JSON.stringify(results,null,2));
 } finally {await browser.close();}
})().catch(e=>{console.error(e);process.exit(1)});

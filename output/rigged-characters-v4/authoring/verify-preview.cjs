const fs=require('node:fs');
const path=require('node:path');
const {chromium}=require('C:/Users/DineshNikam/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright');
async function main(){
  const output=path.resolve(process.argv[2]);
  const browser=await chromium.launch({executablePath:'C:/Users/DineshNikam/AppData/Local/Google/Chrome/Application/chrome.exe',headless:true});
  try{
    const page=await browser.newPage({viewport:{width:1000,height:1000}});
    const errors=[];
    page.on('pageerror',e=>errors.push(e.message));
    await page.goto('http://127.0.0.1:1422/preview.html',{waitUntil:'load'});
    await page.waitForFunction(()=>document.querySelector('#status').dataset.loaded!==undefined,null,{timeout:60000});
    const qa=await page.evaluate(()=>window.MODEL_QA);
    if(!qa?.vrm||qa.skinned<1||qa.clips.length!==15)throw new Error(await page.locator('#status').innerText());
    const captures=[];
    for(const state of ['idle','working','success','sleeping']){
      await page.locator('#animation').selectOption(state);
      await page.waitForTimeout(350);
      const file=path.join(output,`runtime-${state}.png`);
      await page.screenshot({path:file});captures.push(file);
    }
    await page.locator('#skeleton').check();
    await page.screenshot({path:path.join(output,'runtime-skeleton.png')});
    if(errors.length)throw new Error(errors.join('\n'));
    fs.writeFileSync(path.join(output,'runtime-validation.json'),JSON.stringify({schemaVersion:1,status:'passed',engine:'Chrome WebGL + three-vrm',...qa,errors,captures},null,2));
    console.log(JSON.stringify(qa));
  }finally{await browser.close();}
}
main().catch(e=>{console.error(e);process.exitCode=1;});

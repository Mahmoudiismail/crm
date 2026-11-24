
(async function(timeoutMinutes, reportName) {
 const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms)), logs=[];
 const deadline=Date.now()+timeoutMinutes*60000, pollMs=1000, requiredStableSamples=3;
 const loaderSelector='#loader_svg, .loading-screen-wrapper, mat-progress-bar, .dx-loadpanel, .dx-datagrid-loading';
 function isVisible(el) {
  if(!el)return false; const view=el.ownerDocument&&el.ownerDocument.defaultView; if(!view)return false;
  for(let n=el;n&&n.nodeType===1;n=n.parentElement){const s=view.getComputedStyle(n);if(s.display==='none'||s.visibility==='hidden'||s.visibility==='collapse'||Number(s.opacity)===0)return false;}
  return Array.from(el.getClientRects()).some(r=>r.width>0&&r.height>0);
 }
 function findLoader(){return Array.from(document.querySelectorAll(loaderSelector)).find(isVisible)||null;}
 function findExportButton(){
  const found=[], labelled=document.querySelector('div[aria-label="Export"], button[aria-label="Export"]');
  if(labelled)found.push(labelled);
  for(const el of document.querySelectorAll('.dx-button-text'))if((el.textContent||'').trim().toLowerCase()==='export'){const b=el.closest('.dx-button, div[role="button"], button');if(b&&!found.includes(b))found.push(b);}
  for(const el of document.querySelectorAll('button, div[role="button"], .dx-button'))if((el.textContent||'').trim().toLowerCase()==='export'&&!found.includes(el))found.push(el);
  return found.find(isVisible)||null;
 }
 function enabled(b){
  if(!b||!isVisible(b))return false;
  if(['disabled','disable','dx-state-disabled','mat-button-disabled'].some(c=>b.classList.contains(c)))return false;
  if(b.disabled===true||b.getAttribute('disabled')!==null||b.getAttribute('aria-disabled')==='true')return false;
  return b.ownerDocument.defaultView.getComputedStyle(b).pointerEvents!=='none';
 }
 function inspectReport(){
  const grid=document.querySelector('.dx-datagrid');
  const rows=grid?Array.from(grid.querySelectorAll('.dx-datagrid-rowsview .dx-data-row')).filter(isVisible):[];
  const empty=grid?Array.from(grid.querySelectorAll('.dx-datagrid-nodata, [role="status"]')).find(e=>isVisible(e)&&/no data|no records|no results|لا توجد بيانات/i.test(e.textContent||'')):null;
  const busy=!!(grid&&(grid.getAttribute('aria-busy')==='true'||Array.from(grid.querySelectorAll('[aria-busy="true"]')).some(isVisible)));
  const loader=!!findLoader(), button=findExportButton(), rowCount=rows.length, explicitEmpty=!!empty&&rowCount===0;
  const gridHasResult=!!grid&&(rowCount>0||explicitEmpty), exportEnabled=enabled(button);
  return {gridFound:!!grid,rowCount,explicitEmpty,gridBusy:busy,loaderVisible:loader,exportFound:!!button,exportEnabled,gridHasResult,ready:gridHasResult&&!busy&&!loader&&exportEnabled};
 }
 logs.push("Waiting for report grid readiness and stable results. Report: "+reportName);
 logs.push("Zero rows are accepted only with an explicit empty-results state.");
 let previous=null, stableSamples=0, lastLogged=null;
 while(Date.now()<=deadline){
  const state=inspectReport(), signature=JSON.stringify({gridFound:state.gridFound,rowCount:state.rowCount,explicitEmpty:state.explicitEmpty,gridBusy:state.gridBusy,loaderVisible:state.loaderVisible,exportEnabled:state.exportEnabled});
  stableSamples=signature===previous&&state.ready?stableSamples+1:(state.ready?1:0); previous=signature;
  if(signature!==lastLogged){logs.push("Report readiness: "+signature+" stable_samples="+stableSamples);lastLogged=signature;}
  if(state.ready&&stableSamples>=requiredStableSamples){
   const finalState=inspectReport();
   if(finalState.ready&&finalState.rowCount===state.rowCount&&finalState.explicitEmpty===state.explicitEmpty){
    const button=findExportButton(); if(enabled(button)){button.click();logs.push("Grid stable for "+requiredStableSamples+" samples; clicked Export. row_count="+finalState.rowCount+" explicit_empty="+finalState.explicitEmpty);
     return JSON.stringify({status:"SUCCESS",row_count:finalState.rowCount,explicit_empty:finalState.explicitEmpty,logs});}
   }
   stableSamples=0;previous=null;
  }
  await sleep(pollMs);
 }
 const finalState=inspectReport();
 return JSON.stringify({status:"ERROR",msg:"Report grid did not reach a stable, exportable state before timeout.",diagnostics:finalState,logs:logs.concat(["Timeout diagnostics: report="+reportName+" final_state="+JSON.stringify(finalState)])});
})(__TIMEOUT_MINUTES__, __REPORT_NAME_JSON__);

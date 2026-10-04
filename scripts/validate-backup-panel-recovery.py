"""T2-UI1: actual App timer, Vue BackupPanel and Pinia, isolated mock IPC.
Requires Vite on :1420. This is browser evidence, not native SQL/I/O acceptance.
"""
from pathlib import Path
import json, os
from playwright.sync_api import sync_playwright

root = Path(__file__).resolve().parents[1]
out = root / os.environ.get('MANGAFOLIO_BACKUP_UI_RESULT_DIR', 'docs/third-phase-t2-ui1-results/browser')
out.mkdir(parents=True, exist_ok=True)
fixture = {'__file__': str(root / 'scripts/validate-next-phase.py')}
exec(compile((root / 'scripts/validate-next-phase.py').read_text().split('checks=[];errors=[];contrast=[]')[0], fixture['__file__'], 'exec'), fixture)
script = fixture['script'] + r'''
window.backupConfig={enabled:true,retention:1,lastSuccess:1700000000000,lastError:''};
window.backupResponses=[];window.localFailure='';
const baseInvoke=__TAURI_INTERNALS__.invoke;
__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 if(['get_backup_settings','set_backup_settings','run_automatic_backup'].includes(cmd))calls.push({cmd,args});
 if(cmd==='get_backup_settings')return structuredClone(backupConfig);
 if(cmd==='set_backup_settings'){Object.assign(backupConfig,args);return structuredClone(backupConfig);}
 if(cmd==='run_automatic_backup'){
  await new Promise(r=>setTimeout(r,30));
  const response=backupResponses.shift()||{lastError:''};
  backupConfig.lastError=response.lastError;
  if(response.reject)throw response.lastError;
  return structuredClone(backupConfig);
 }
 if(cmd==='preview_library_backup'&&localFailure==='preview')throw 'TEST 預覽讀取失敗';
 if(cmd==='restore_library_backup'&&localFailure==='restore')throw 'TEST 還原交易失敗';
 if(cmd==='export_library_backup'&&localFailure==='export')throw 'TEST 匯出寫入失敗';
 return baseInvoke(cmd,args);
};
const originalSetInterval=window.setInterval.bind(window);
window.setInterval=(callback,delay,...args)=>{
 if(delay===900000)window.testBackupTick=()=>callback(...args);
 return originalSetInterval(callback,delay,...args);
};
'''
checks = []; errors = []
with sync_playwright() as p:
    browser = p.chromium.launch(headless=True, executable_path=os.environ.get('CHROMIUM_EXECUTABLE', '/usr/bin/chromium'), args=['--no-sandbox'])
    page = browser.new_page(viewport={'width':1440, 'height':1000})
    page.set_default_timeout(8000)
    page.on('pageerror', lambda e: errors.append(str(e)))
    page.add_init_script(script)
    page.goto('http://127.0.0.1:1420/')
    page.locator('.book-card').nth(3).wait_for()
    page.wait_for_function('typeof testBackupTick === "function"')
    page.get_by_role('button', name='管理與備份', exact=True).click()
    page.get_by_text('備份與還原', exact=True).click()
    panel = page.locator('.backup-panel')
    alerts = panel.locator('[role=alert]')
    page.evaluate('window.originalBackupPanel=document.querySelector(".backup-panel")')

    def state():
        return page.evaluate('''async()=>{
          const {useBackupStore}=await import('/src/stores/backup.ts');
          const {useLibraryStore}=await import('/src/stores/library.ts');
          const b=useBackupStore();
          return {pending:b.pending,error:b.error,lastError:b.settings.lastError,lastSuccess:b.settings.lastSuccess,
            panelAlert:document.querySelector('.backup-panel [role=alert]')?.textContent?.trim()||'',
            notice:useLibraryStore().importNotice};
        }''')

    def idle():
        page.wait_for_function('''async()=>{
          const {useBackupStore}=await import('/src/stores/backup.ts');
          const {useLibraryStore}=await import('/src/stores/library.ts');
          return !useBackupStore().pending&&!useLibraryStore().managing;
        }''')
        page.wait_for_timeout(50)

    def tick(response):
        before = page.evaluate('calls.filter(c=>c.cmd==="run_automatic_backup").length')
        page.evaluate('(r)=>{backupResponses.push(r);testBackupTick();}', response)
        idle()
        assert page.evaluate('calls.filter(c=>c.cmd==="run_automatic_backup").length') == before + 1
        assert page.evaluate('backupResponses.length') == 0

    try:
        idle()
        first='備份已建立，但保留清理失敗；下次檢查會重試：cleanup failed'
        second='備份狀態恢復／清理失敗；下次檢查會重試：cleanup failed'
        page.evaluate('(r)=>backupResponses.push(r)', {'lastError':first,'reject':True})
        page.get_by_role('button', name='立即建立本機安全備份', exact=True).click()
        idle(); assert first in alerts.inner_text()
        assert state()['notice'] != '本機安全備份已建立。'
        checks.append({'case':'immediate-failure-visible','state':state()})
        tick({'lastError':second,'reject':True})
        retry_state = state()
        checks.append({'case':'scheduled-retry-failure-replaces-current-error','state':retry_state})
        tick({'lastError':''})
        assert alerts.count()==0, state()
        assert state()['error']=='' and state()['lastError']==''
        assert second in retry_state['panelAlert'], retry_state
        assert state()['lastSuccess']==1700000000000
        assert page.evaluate('originalBackupPanel===document.querySelector(".backup-panel")')
        checks.append({'case':'scheduled-recovery-clears-panel-without-remount','state':state()})
        page.screenshot(path=str(out/'recovered-panel.png'))
        # A successful check can legitimately return an unretried creation error.
        creation='備份擁有權登記失敗；未建立完整新備份：manifest failed'
        page.evaluate('(r)=>backupResponses.push(r)', {'lastError':creation,'reject':True})
        page.get_by_role('button', name='立即建立本機安全備份', exact=True).click(); idle()
        tick({'lastError':creation})
        assert creation in alerts.inner_text(), state()
        assert state()['error']=='' and state()['lastError']==creation
        assert state()['notice'] != '本機安全備份已建立。'
        checks.append({'case':'unretried-creation-error-remains-visible','state':state()})
        tick({'lastError':''}); assert alerts.count()==0
        # Local export, preview and restore errors belong to their own operations.
        for kind, message in [('export','TEST 匯出寫入失敗'),('preview','TEST 預覽讀取失敗'),('restore','TEST 還原交易失敗')]:
            page.evaluate('(v)=>localFailure=v',kind)
            if kind=='export':
                page.get_by_role('button', name='匯出書庫備份', exact=True).click()
            else:
                page.get_by_role('button', name='還原書庫備份', exact=True).click(); idle()
                if kind=='restore': page.get_by_role('button', name='確認合併還原', exact=True).click()
            idle(); assert message in alerts.inner_text()
            before=state()['panelAlert']; tick({'lastError':''})
            assert state()['panelAlert']==before
            assert state()['error']=='' and state()['lastError']==''
            checks.append({'case':kind+'-error-survives-scheduled-success','state':state()})
        page.evaluate('localFailure=""')
        page.get_by_role('button', name='立即建立本機安全備份', exact=True).click(); idle()
        assert alerts.count()==0
        assert state()['notice']=='本機安全備份已建立。'
        checks.append({'case':'successful-immediate-backup-shows-success-notice','state':state()})
        assert page.evaluate('originalBackupPanel===document.querySelector(".backup-panel")')
        assert not errors, errors
        force_calls = page.evaluate('calls.filter(c=>c.cmd==="run_automatic_backup").map(c=>c.args.force)')
        assert True in force_calls and False in force_calls
        assert page.locator('.backup-panel').count()==1
        print(json.dumps({'checks':checks,'pageErrors':errors,'forceCalls':force_calls},ensure_ascii=False))
    finally:
        (out/'results.json').write_text(json.dumps({'mode':'actual Vue/Pinia/App 900000ms callback; mock IPC, not native acceptance','checks':checks,'lastState':state(),'pageErrors':errors},ensure_ascii=False,indent=2)+'\n')
        browser.close()

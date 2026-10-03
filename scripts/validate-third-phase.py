"""Actual Vue browser interaction regression, isolated mock IPC; not native acceptance."""
from pathlib import Path
import os,json,time
from playwright.sync_api import sync_playwright
root=Path(__file__).resolve().parents[1]
out=root/'docs/third-phase-results/browser';images=root/'docs/images/third-phase/implemented'
out.mkdir(parents=True,exist_ok=True);images.mkdir(parents=True,exist_ok=True)
fixture={'__file__':str(root/'scripts/validate-next-phase.py')}
source=(root/'scripts/validate-next-phase.py').read_text().split('checks=[];errors=[];contrast=[]')[0]
exec(compile(source,fixture['__file__'],'exec'),fixture)
script=fixture['script']+r'''
window.testBooks=[];let id=1;
for(const [series,count] of [['海風紀事',10],['森林郵局',6],['星軌旅人',8],['雨巷食堂',4]])for(let vol=1;vol<=count;vol++,id++){
 const b=makeBook(id,series+' '+String(vol).padStart(2,'0'));Object.assign(b,{series,volume:String(vol),notes:'隔離示例資料：使用者備註。',available:id!==28,tags:[testTags[0]],readingStatus:'unread',lastIndex:0,lastReadAt:null,favorite:id===3});
 if(id===1||id===2||id===11||id===12||id===17)Object.assign(b,{readingStatus:'read',lastIndex:119,lastReadAt:100});
 if(id===3)Object.assign(b,{readingStatus:'reading',lastIndex:81,lastReadAt:500});testBooks.push(b);
}
window.testMarks=[{id:1,bookId:3,pageName:'082.png',pageIndex:81,name:'港口',note:'純文字頁面筆記'}];
window.backupConfig={enabled:false,retention:5,lastSuccess:null,lastError:''};window.failMark=false;window.failSeries=false;window.missingBookmark=false;window.failOpen=false;
window.eventCallbacks={};let callbackId=100;
window.__TAURI_INTERNALS__.transformCallback=fn=>{const id=++callbackId;eventCallbacks[id]=fn;return id;};
const oldInvoke=window.__TAURI_INTERNALS__.invoke;
window.__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 if(cmd==='plugin:event|listen'&&args.event==='tauri://close-requested'){window.closeHandler=eventCallbacks[args.handler];return 1;}
 if(['assign_book_series','list_bookmarks','save_bookmark','delete_bookmark','get_backup_settings','set_backup_settings','run_automatic_backup','preview_library_backup','open_library_book','open_bookmark','render_page'].includes(cmd))calls.push({cmd,args});
 if(cmd==='assign_book_series'){if(window.failSeries)throw 'TEST series transaction failed';testBooks.forEach(b=>{if(args.ids.includes(b.id))b.series=args.name.trim();});return;}
 if(cmd==='list_bookmarks')return structuredClone(testMarks.filter(m=>m.bookId===args.bookId));
 if(cmd==='save_bookmark'){if(window.failMark)throw 'TEST bookmark failed';const mark=structuredClone(args.bookmark);if(!mark.id){mark.id=Math.max(0,...testMarks.map(m=>m.id))+1;testMarks.push(mark);}else Object.assign(testMarks.find(m=>m.id===mark.id),{name:mark.name,note:mark.note});return mark;}
 if(cmd==='delete_bookmark'){testMarks=testMarks.filter(m=>m.id!==args.id);return;}
 if(cmd==='get_backup_settings')return structuredClone(backupConfig);
 if(cmd==='set_backup_settings'){Object.assign(backupConfig,args);return structuredClone(backupConfig);}
 if(cmd==='run_automatic_backup'){if(args.force||backupConfig.enabled)backupConfig.lastSuccess=Date.now();return structuredClone(backupConfig);}
 if(cmd==='preview_library_backup'&&window.unsupportedBackup)return {added:0,skipped:0,conflicts:0,unsupported:1,issues:['不支援較新版備份 v4'],canRestore:false,version:4};
 if(cmd==='preview_library_backup')return {added:2,skipped:28,conflicts:0,unsupported:0,issues:[],canRestore:true,version:3};
 if(cmd==='open_library_book'||cmd==='open_bookmark'){
  if(window.holdOpen)await new Promise(resolve=>window.finishOpen=resolve);
  if(window.failOpen)throw 'TEST offline source';if(cmd==='open_bookmark'&&window.missingBookmark)throw '書籤頁面已不存在，未跳轉';
  const b=testBooks.find(b=>b.id===(args.bookId??args.id));const pages=Array.from({length:120},(_,i)=>String(i+1).padStart(3,'0')+'.png');
  return {bookId:b.id,sessionId:Date.now(),favorite:b.favorite,title:b.title,pageCount:120,pages,startIndex:cmd==='open_bookmark'?pages.indexOf(testMarks.find(m=>m.id===args.id).pageName):b.lastIndex,preferences:b.preferences};
 }
 if(cmd==='render_page')return Uint8Array.from(atob(COVER),c=>c.charCodeAt(0)).buffer;
 return oldInvoke(cmd,args);
};
'''.replace('COVER',json.dumps(fixture['covers'][0]))
checks=[];errors=[]
with sync_playwright() as p:
 browser=p.chromium.launch(headless=True,executable_path=os.environ.get('CHROMIUM_EXECUTABLE','/usr/bin/chromium'),args=['--no-sandbox'])
 page=browser.new_page(viewport={'width':1487,'height':1058});page.set_default_timeout(10000)
 page.on('pageerror',lambda e:errors.append(str(e)));page.add_init_script(script);page.goto('http://127.0.0.1:1420/')
 page.locator('.series-card').nth(3).wait_for()
 # All previews use the same books and real components.
 for style,theme in [('calm','light'),('catalog','light'),('night','dark')]:
  page.get_by_text('外觀設定',exact=True).click();page.get_by_label('介面風格',exact=True).select_option(style);page.get_by_label('明暗主題',exact=True).select_option(theme);page.get_by_label('書庫檢視',exact=True).select_option('grid');page.get_by_text('外觀設定',exact=True).click()
  page.locator('.library-content').evaluate('(e)=>e.scrollTop=0');page.screenshot(path=str(images/f'{style}-home.png'))
  page.get_by_role('button',name='開啟系列：海風紀事',exact=True).click()
  assert page.locator('.book-card').count()==10
  page.get_by_role('searchbox',name='搜尋書名',exact=True).fill('03');assert page.locator('.book-card').count()==1
  page.get_by_text('外觀設定',exact=True).click()
  for view in ['grid','detail','compact']:
   page.get_by_label('書庫檢視',exact=True).select_option(view);assert page.locator('.book-card').count()==1;assert page.get_by_role('searchbox',name='搜尋書名').input_value()=='03'
  page.get_by_label('書庫檢視',exact=True).select_option('detail');page.get_by_text('外觀設定',exact=True).click()
  page.get_by_role('searchbox',name='搜尋書名').fill('');page.locator('.library-content').evaluate('(e)=>e.scrollTop=0');page.screenshot(path=str(images/f'{style}-series.png'))
  page.get_by_role('button',name='查看資訊：海風紀事 03',exact=True).click();page.get_by_role('dialog',name='海風紀事 03',exact=True).wait_for();page.screenshot(path=str(images/f'{style}-book-details.png'))
  # A retained detail component must refresh its cover when the source becomes available again.
  page.evaluate('''async()=>{const {useLibraryStore}=await import('/src/stores/library.ts');useLibraryStore().books.find(b=>b.id===3).available=false;}''');page.locator('.detail-panel .book-cover img').wait_for(state='detached');page.evaluate('''async()=>{const {useLibraryStore}=await import('/src/stores/library.ts');useLibraryStore().books.find(b=>b.id===3).available=true;}''');page.locator('.detail-panel .book-cover img').wait_for()
  page.get_by_role('button',name='關閉書籍詳情',exact=True).click();page.get_by_role('button',name='返回書架',exact=True).click()
  # Complete companion screens use identical components/data in each style.
  page.get_by_role('button',name='管理與備份',exact=True).click();page.get_by_text('備份與還原',exact=True).click();page.get_by_role('button',name='還原書庫備份',exact=True).click();page.get_by_role('region',name='還原預覽').scroll_into_view_if_needed();page.screenshot(path=str(images/f'{style}-backup-preview.png'))
  page.get_by_role('button',name='取消還原',exact=True).click();page.evaluate('window.unsupportedBackup=true');page.get_by_role('button',name='還原書庫備份',exact=True).click();assert page.get_by_role('button',name='確認合併還原',exact=True).is_disabled();page.get_by_role('region',name='還原預覽').scroll_into_view_if_needed();page.screenshot(path=str(images/f'{style}-backup-rejected.png'));page.get_by_role('button',name='取消還原',exact=True).click();page.evaluate('window.unsupportedBackup=false');page.get_by_role('button',name='結束管理',exact=True).click()
  page.get_by_role('searchbox',name='搜尋書名',exact=True).fill('查無此書測試');page.screenshot(path=str(images/f'{style}-no-results.png'));page.get_by_role('searchbox',name='搜尋書名',exact=True).fill('')
  page.get_by_role('button',name='查看資訊：海風紀事 03',exact=True).click();page.get_by_role('button',name='閱讀',exact=True).click();page.locator('.page').first.wait_for();page.mouse.move(750,500);page.wait_for_timeout(220);page.screenshot(path=str(images/f'{style}-reader-hidden.png'));page.mouse.move(750,2);page.wait_for_timeout(220);page.screenshot(path=str(images/f'{style}-reader-shown.png'));page.get_by_role('button',name='← 書庫',exact=True).click();page.locator('.series-card').first.wait_for()
  page.set_viewport_size({'width':640,'height':480});page.get_by_role('button',name='書庫導覽與設定',exact=True).click();page.screenshot(path=str(images/f'{style}-narrow-home.png'));page.get_by_role('button',name='書庫導覽與設定',exact=True).click();page.get_by_role('button',name='開啟系列：海風紀事',exact=True).click();page.get_by_role('button',name='查看資訊：海風紀事 03',exact=True).click();page.screenshot(path=str(images/f'{style}-narrow-details.png'));page.get_by_role('button',name='關閉書籍詳情',exact=True).click();page.get_by_role('button',name='返回書架',exact=True).click();page.set_viewport_size({'width':1487,'height':1058});page.locator('.library-content').evaluate('(e)=>e.scrollTop=0')
 checks.append('three styles, cover refresh after offline/online, and series shared views/search; same data screenshots of home, series and book details')
 # Homepage search/sort/scroll survive a series excursion.
 page.get_by_role('searchbox',name='搜尋書名').fill('海風');page.get_by_label('書籍排序',exact=True).select_option('title');page.locator('.library-content').evaluate('(e)=>e.scrollTop=80');page.wait_for_timeout(80)
 page.get_by_role('button',name='開啟系列：海風紀事',exact=True).click();page.get_by_role('searchbox',name='搜尋書名').fill('02');page.get_by_role('button',name='返回書架',exact=True).click()
 assert page.get_by_role('searchbox',name='搜尋書名').input_value()=='海風';assert page.get_by_label('書籍排序').input_value()=='title';assert abs(page.locator('.library-content').evaluate('(e)=>e.scrollTop')-80)<2
 checks.append('return restores homepage search, sort and scroll')
 # Transaction errors do not alter mock data; real SQL rollback tested in Rust.
 page.get_by_role('button',name='管理與備份',exact=True).click();page.get_by_role('checkbox',name='選取：海風紀事 01',exact=True).check();page.get_by_label('批次系列',exact=True).fill('新系列');page.evaluate('window.failSeries=true');page.get_by_role('button',name='指定系列',exact=True).click();page.get_by_text('TEST series transaction failed',exact=False).wait_for();assert page.evaluate('testBooks[0].series')=='海風紀事'
 page.evaluate('window.failSeries=false');page.get_by_role('button',name='指定系列',exact=True).click();page.get_by_text('系列歸屬已更新，書籍與來源保留。',exact=True).wait_for();assert page.evaluate('testBooks[0].series')=='新系列'
 page.get_by_text('備份與還原',exact=True).click();page.get_by_role('button',name='還原書庫備份',exact=True).click();page.get_by_role('region',name='還原預覽').wait_for();assert not page.evaluate('calls.some(c=>c.cmd==="restore_library_backup")');page.locator('.library-content').evaluate('(e)=>e.scrollTop=0');page.screenshot(path=str(images/'backup-preview.png'))
 page.get_by_role('checkbox',name='自動備份',exact=False).check();page.get_by_label('保留份數',exact=True).select_option('3');assert page.evaluate('backupConfig.retention')==3
 page.get_by_role('button',name='取消還原',exact=True).click();page.get_by_role('button',name='結束管理',exact=True).click();page.get_by_role('searchbox',name='搜尋書名').fill('')
 checks.append('series batch failure/success; read-only restore preview and auto-backup preference controls')
 # Real store, native IPC mocked: bookmarks remain plain text, and missing pages keep reader state.
 page.get_by_role('button',name='查看資訊：海風紀事 03',exact=True).click();page.get_by_role('button',name='閱讀',exact=True).click();page.locator('.page').first.wait_for();page.mouse.move(750,500);page.wait_for_timeout(250);page.screenshot(path=str(images/'reader-hidden.png'))
 page.mouse.move(750,2);page.get_by_role('button',name='書籤／筆記',exact=True).click();page.get_by_label('書籤名稱',exact=True).fill('新的標記');page.get_by_label('簡短筆記',exact=True).fill('<img id="injected" src=x onerror=alert(1)>')
 page.evaluate('window.failMark=true');page.get_by_role('button',name='新增書籤',exact=True).click();page.get_by_text('TEST bookmark failed',exact=True).wait_for();assert page.evaluate('testMarks.length')==1
 page.evaluate('window.failMark=false');page.get_by_role('button',name='新增書籤',exact=True).click();page.wait_for_function('testMarks.length===2');assert page.locator('#injected').count()==0;page.screenshot(path=str(images/'reader-bookmarks.png'))
 page.evaluate('window.missingBookmark=true');before=page.evaluate('''async()=>{const {useReaderStore}=await import('/src/stores/reader.ts');const r=useReaderStore();return [r.bookId,r.index,r.sessionId]}''');page.get_by_role('button',name='港口',exact=False).click();page.get_by_text('書籤頁面已不存在，未跳轉',exact=False).first.wait_for();after=page.evaluate('''async()=>{const {useReaderStore}=await import('/src/stores/reader.ts');const r=useReaderStore();return [r.bookId,r.index,r.sessionId]}''');assert before==after
 page.evaluate('window.missingBookmark=false');page.get_by_role('button',name='關閉書籤',exact=True).click();page.get_by_role('checkbox',name='固定工具列',exact=True).check();page.mouse.move(750,500);page.wait_for_timeout(250);assert page.locator('.reader-edge.top .reader-chrome').evaluate('(e)=>getComputedStyle(e).opacity')=='1';page.screenshot(path=str(images/'reader-pinned.png'))
 # Real App close-request handler must keep window alive during an in-flight book switch.
 page.evaluate('''async()=>{window.holdOpen=true;window.failOpen=true;const {useReaderStore}=await import('/src/stores/reader.ts');void useReaderStore().openBook(4);}''');page.wait_for_function('typeof window.finishOpen==="function"');page.evaluate('window.closeHandler({event:"tauri://close-requested",id:1,payload:null})');assert not page.evaluate('calls.some(c=>c.cmd==="plugin:window|destroy")');page.get_by_text('書庫或閱讀操作進行中，請完成後再關閉視窗。',exact=False).wait_for();page.evaluate('window.holdOpen=false;window.finishOpen()');page.wait_for_function('''async()=>{const {useReaderStore}=await import('/src/stores/reader.ts');return !useReaderStore().loading}''');page.evaluate('window.failOpen=false')
 checks.append('actual App close-request callback prevents window destruction during an in-flight failed book switch')
 page.evaluate('''async()=>{const {useReaderStore}=await import('/src/stores/reader.ts');await useReaderStore().goto(119)}''');page.get_by_role('button',name='閱讀下一集：4',exact=True).wait_for();page.evaluate('window.failOpen=true');page.get_by_role('button',name='閱讀下一集：4',exact=True).click();page.get_by_text('TEST offline source',exact=False).wait_for();assert page.evaluate('''async()=>{const {useReaderStore}=await import('/src/stores/reader.ts');return useReaderStore().bookId}''')==3
 page.evaluate('window.failOpen=false');page.get_by_role('button',name='閱讀下一集：4',exact=True).click();page.wait_for_function('''async()=>{const {useReaderStore}=await import('/src/stores/reader.ts');return useReaderStore().bookId===4}''')
 checks.append('bookmarks fail/retry, plain text, missing-page preservation, pinned toolbar, last-page next volume failure/success')
 page.get_by_role('button',name='← 書庫',exact=True).click();page.locator('.series-card').first.wait_for();page.set_viewport_size({'width':640,'height':480});page.get_by_role('button',name='書庫導覽與設定',exact=True).click();assert page.evaluate('document.documentElement.scrollWidth<=innerWidth');page.screenshot(path=str(images/'narrow-home.png'))
 page.get_by_role('button',name='書庫導覽與設定',exact=True).click();page.get_by_role('button',name='開啟系列：海風紀事',exact=True).click();page.get_by_role('button',name='查看資訊：海風紀事 03',exact=True).click();assert page.get_by_role('button',name='關閉書籍詳情',exact=True).is_visible();page.screenshot(path=str(images/'narrow-book-details.png'))
 checks.append('640x480 navigation, series and book detail controls accessible without horizontal overflow')
 # Series aggregation and bounded rendering with 10,000 isolated books.
 page.get_by_role('button',name='關閉書籍詳情',exact=True).click();page.set_viewport_size({'width':1487,'height':1058})
 perf=page.evaluate("""async()=>{const {groupSeries}=await import('/src/lib/series.ts');const books=Array.from({length:10000},(_,i)=>({...testBooks[0],id:1000+i,series:'量測系列 '+Math.floor(i/10),volume:String(i%10+1)}));const start=performance.now();const groups=groupSeries(books);const aggregateMs=performance.now()-start;const {useLibraryStore}=await import('/src/stores/library.ts');const s=useLibraryStore();s.books=books;s.query='';s.section='series';s.activeSeries=null;return {books:books.length,groups:groups.length,aggregateMs}}""")
 page.wait_for_function('document.querySelectorAll(".series-card").length===60');perf['renderedSeries']=page.locator('.series-card').count();assert perf['groups']==1000
 checks.append('10,000-book series aggregation measured; series covers initially bounded to 60')
 assert not errors,errors
 browser.close()
result={'exitCode':0,'checks':checks,'pageErrors':errors,'performance':perf,'boundary':'Actual Vue browser / in-memory isolated mock IPC; not native Windows acceptance'}
(out/'results.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n');print(json.dumps(result,ensure_ascii=False,indent=2))

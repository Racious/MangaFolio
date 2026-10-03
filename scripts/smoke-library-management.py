# Management UI smoke: run against an existing Vite dev server on port 1420.
# Requires Python + playwright + its Chromium browser (see docs/validation.md).
# Tauri IPC is mocked; this does not replace native desktop validation.
# Uses in-memory fixtures and never modifies an actual library or manga files.
from playwright.sync_api import sync_playwright
import os
cover='iVBORw0KGgoAAAANSUhEUgAAAAIAAAADCAIAAAA2iEnWAAAAFUlEQVR4nGO8NM2NgYGBiYGBAUEBACDKAbQKAJWiAAAAAElFTkSuQmCC'
script=r'''
 localStorage.setItem('mangafolio.library-guide.v1.hidden','true');
 const prefs = {direction:'rtl',pageMode:'single',zoom:'window',fixedScale:1,doubleCover:false,transition:'book'};
 window.testBooks = [1,2].map(id=>({id,path:'/books/'+id+'.cbz',title:'Test '+id,format:'cbz',pageCount:3,favorite:false,lastIndex:1,lastPageName:'2.png',lastReadAt:id*100,preferences:prefs,available:true}));
 window.calls=[]; window.confirmResult=true; window.failRemoval=false; window.failRestore=false;
 window.__TAURI_INTERNALS__ = {
 metadata: {currentWindow:{label:'main'}, currentWebview:{label:'main'}}, transformCallback:()=>1, unregisterCallback:()=>{},
 invoke: async (cmd,args={}) => {
  window.calls.push({cmd,args});
  if(cmd==='list_library') return structuredClone(window.testBooks);
  if(cmd==='library_cover') return Uint8Array.from(atob('COVER'), c=>c.charCodeAt(0)).buffer;
  if(cmd==='plugin:app|version') return '0.1.4';
  if(cmd==='plugin:updater|check') return null;
  if(cmd==='plugin:event|listen') return 1;
  if(cmd==='plugin:dialog|message') return window.confirmResult ? (args.buttons.OkCancelCustom?.[0] || 'Yes') : (args.buttons.OkCancelCustom?.[1] || 'No');
  if(cmd==='plugin:dialog|save') return '/backup.json';
  if(cmd==='plugin:dialog|open') return '/moved/book.cbz';
  if(cmd==='favorite_library_books') window.testBooks.forEach(b=>{if(args.ids.includes(b.id))b.favorite=args.favorite});
  if(cmd==='remove_library_books') {
   if(window.failRemoval) throw 'TEST: deletion failed';
   window.testBooks=window.testBooks.filter(b=>!args.ids.includes(b.id));
  }
  if(cmd==='relink_library_book') { const b=window.testBooks.find(b=>b.id===args.id); b.path=args.path; return b; }
  if(cmd==='restore_library_backup') {
   if(window.failRestore) throw 'TEST: invalid backup';
   window.testBooks.push({id:3,path:'/books/3.cbz',title:'Test 3',format:'cbz',pageCount:3,favorite:true,lastIndex:1,lastPageName:'2.png',lastReadAt:300,preferences:prefs,available:false});
   return {added:1,skipped:2};
  }
  return null;
 }
 };
'''.replace('COVER',cover)
with sync_playwright() as p:
 launch_options = {'headless': True}
 if os.environ.get('CHROMIUM_EXECUTABLE'):
  launch_options['executable_path'] = os.environ['CHROMIUM_EXECUTABLE']
 browser=p.chromium.launch(**launch_options)
 page=browser.new_page(viewport={'width':1280,'height':900})
 page.set_default_timeout(5000)
 page.add_init_script(script)
 page.goto('http://127.0.0.1:1420/')
 page.get_by_role('button',name='管理與備份',exact=True).click()
 page.get_by_role('button',name='選取目前顯示的 2 本',exact=True).click()
 page.get_by_role('button',name='批次收藏',exact=True).click()
 page.get_by_text('已收藏 2 本書。',exact=True).wait_for()
 assert page.evaluate('window.testBooks.every(b=>b.favorite)')
 assert not page.get_by_role('checkbox').first.is_checked()
 page.get_by_role('checkbox',name='選取：Test 1',exact=True).check()
 page.get_by_role('searchbox',name='搜尋書名',exact=True).fill('Test 1')
 assert not page.get_by_role('checkbox').first.is_checked()
 page.get_by_role('searchbox',name='搜尋書名',exact=True).fill('')
 # Seed a loaded book to verify old active progress is flushed and discarded after mutations.
 page.evaluate("""async ()=>{const {useReaderStore}=await import('/src/stores/reader.ts'); const r=useReaderStore();r.bookId=1;r.pages=['1.png','2.png','3.png'];r.title='Test 1';r.index=1;window.readerTest=r;}""")
 page.get_by_role('checkbox',name='選取：Test 1',exact=True).check()
 page.evaluate('window.confirmResult=false; window.calls=[]')
 page.get_by_role('button',name='移除所選',exact=True).click()
 page.wait_for_function('!window.readerTest.savingProgress && window.calls.some(c=>c.cmd==="plugin:dialog|message")')
 assert page.evaluate('!window.calls.some(c=>c.cmd==="remove_library_books")')
 page.evaluate('window.confirmResult=true; window.failRemoval=true')
 page.get_by_role('button',name='移除所選',exact=True).click()
 page.get_by_text('TEST: deletion failed',exact=False).wait_for()
 assert page.evaluate('window.readerTest.bookId===1')
 page.evaluate('window.failRemoval=false;window.calls=[]')
 page.get_by_role('button',name='移除所選',exact=True).click()
 page.get_by_text('已移除 1 本書，原始漫畫檔案已保留。',exact=True).wait_for()
 assert page.evaluate('window.readerTest.bookId===null && !window.readerTest.hasBook')
 assert page.evaluate('window.calls.findIndex(c=>c.cmd==="save_reading_progress") < window.calls.findIndex(c=>c.cmd==="remove_library_books")')
 page.get_by_role('checkbox',name='選取：Test 2',exact=True).check()
 page.get_by_role('button',name='重新指定 ZIP／CBZ',exact=True).click()
 page.get_by_text('已更新來源，請從封面重新開啟閱讀。',exact=True).wait_for()
 assert page.evaluate('window.testBooks[0].path==="/moved/book.cbz"')
 page.get_by_role('button',name='匯出書庫備份',exact=True).click()
 page.get_by_text('備份已匯出：',exact=False).wait_for()
 page.evaluate('window.failRestore=true')
 page.get_by_role('button',name='還原書庫備份',exact=True).click()
 page.get_by_text('TEST: invalid backup',exact=False).wait_for()
 assert not page.get_by_text('還原完成：',exact=False).count()
 page.evaluate('window.failRestore=false')
 page.get_by_role('button',name='還原書庫備份',exact=True).click()
 page.get_by_text('還原完成：加入 1 本，略過 2 本既有來源。',exact=True).wait_for()
 page.set_viewport_size({'width':640,'height':480})
 assert page.evaluate('document.querySelector(".library-view").scrollWidth <= document.querySelector(".library-view").clientWidth')
 if os.environ.get('SMOKE_SCREENSHOT_PATH'):
  page.screenshot(path=os.environ['SMOKE_SCREENSHOT_PATH'])
 browser.close()
 print('Management UI smoke passed: batch selection/favorites, filter clearing, canceled/failed/successful removal, active-reader flush/discard, relink, export, invalid/valid restore, 640px layout. Tauri IPC was mocked.')

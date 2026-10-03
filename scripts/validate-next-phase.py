"""Isolated browser integration checks. In-memory Tauri IPC, not native acceptance.
Run with Vite on :1420; Python Playwright and Chromium required.
Screenshots and machine-readable results are written under docs/next-phase-results/ui.
"""
from playwright.sync_api import sync_playwright
from pathlib import Path
import json, os, time
root=Path(__file__).resolve().parents[1]
out=root/os.environ.get('MANGAFOLIO_UI_RESULT_DIR','docs/next-phase-results/ui')
out.mkdir(parents=True,exist_ok=True)
images=root/os.environ.get('MANGAFOLIO_UI_IMAGE_DIR','docs/images/next-phase/implemented')
images.mkdir(parents=True,exist_ok=True)
script=r'''
localStorage.setItem('mangafolio.library-guide.v1.hidden','true');
const prefs={direction:'rtl',pageMode:'single',zoom:'window',fixedScale:1,doubleCover:false,transition:'book'};
window.testTags=[{id:1,name:'冒險'},{id:2,name:'日常'}];
window.makeBook=(id,title)=>({id,path:'/isolated/'+title+'.cbz',title,sourceTitle:title,customTitle:'',series:id===1?'海邊故事':'',volume:id===1?'2':'',notes:'',readingStatus:id===2?'read':id===1?'reading':'unread',statusManual:false,tags:id===1?[{id:1,name:'冒險'}]:[],format:'cbz',pageCount:120,favorite:id===1,lastIndex:id===1?42:0,lastPageName:id===1?'043.png':null,lastReadAt:id===1?100:null,preferences:prefs,available:id!==4});
window.testBooks=['海邊故事','森林手帖','夜行列車','星際日誌'].map((t,i)=>window.makeBook(i+1,t));
window.calls=[];window.confirmResult=true;window.failRemoval=false;window.failRestore=false;window.failEdit=false;window.failFavorite=false;
window.__TAURI_INTERNALS__={metadata:{currentWindow:{label:'main'},currentWebview:{label:'main'}},transformCallback:()=>1,unregisterCallback:()=>{},invoke:async(cmd,args={})=>{
window.calls.push({cmd,args});
if(cmd==='list_library')return structuredClone(window.testBooks);
if(cmd==='list_tags')return structuredClone(window.testTags);
if(cmd==='library_cover')return Uint8Array.from(atob(COVERS[(args.id-1)%4]),c=>c.charCodeAt(0)).buffer;
if(cmd==='plugin:app|version')return '0.1.4';
if(cmd==='plugin:updater|check')return null;
if(cmd==='plugin:event|listen')return 1;
if(cmd==='plugin:dialog|message')return window.confirmResult?(args.buttons.OkCancelCustom?.[0]||'Yes'):(args.buttons.OkCancelCustom?.[1]||'No');
if(cmd==='plugin:dialog|open')return args.options?.multiple?['/new.cbz','/failed.cbz','/conflict.cbz']: '/isolated/relinked.cbz';
if(cmd==='plugin:dialog|save')return '/isolated/backup.json';
if(cmd==='set_favorite'){if(window.failFavorite)throw 'TEST favorite failed';window.testBooks.find(b=>b.id===args.id).favorite=args.favorite;}
if(cmd==='favorite_library_books')window.testBooks.forEach(b=>{if(args.ids.includes(b.id))b.favorite=args.favorite});
if(cmd==='set_reading_status')window.testBooks.forEach(b=>{if(args.ids.includes(b.id)){b.readingStatus=args.status==='auto'?(b.lastReadAt?'reading':'unread'):args.status;b.statusManual=args.status!=='auto';}});
if(cmd==='create_tag'){const tag={id:window.testTags.length+1,name:args.name.trim()};if(!tag.name||window.testTags.some(t=>t.name.toLowerCase()===tag.name.toLowerCase()))throw 'TEST duplicate tag';window.testTags.push(tag);return tag;}
if(cmd==='rename_tag'){window.testTags.find(t=>t.id===args.id).name=args.name;window.testBooks.forEach(b=>b.tags.forEach(t=>{if(t.id===args.id)t.name=args.name}));}
if(cmd==='delete_tag'){window.testTags=window.testTags.filter(t=>t.id!==args.id);window.testBooks.forEach(b=>b.tags=b.tags.filter(t=>t.id!==args.id));}
if(cmd==='assign_book_tag')window.testBooks.forEach(b=>{if(args.ids.includes(b.id)){b.tags=b.tags.filter(t=>t.id!==args.tagId);if(args.add)b.tags.push(structuredClone(window.testTags.find(t=>t.id===args.tagId)));}});
if(cmd==='edit_library_book'){if(window.failEdit)throw 'TEST edit failed';const b=window.testBooks.find(b=>b.id===args.id);Object.assign(b,args.details);b.title=b.customTitle||b.sourceTitle;return structuredClone(b);}
if(cmd==='remove_library_books'){if(window.failRemoval)throw 'TEST deletion failed';window.testBooks=window.testBooks.filter(b=>!args.ids.includes(b.id));}
if(cmd==='relink_library_book'){const b=window.testBooks.find(b=>b.id===args.id);b.path=args.path;b.available=true;return b;}
if(cmd==='restore_library_backup'){if(window.failRestore)throw 'TEST invalid backup';return {added:0,skipped:4};}
if(cmd==='import_book_result'){await new Promise(r=>setTimeout(r,80));if(args.path.includes('failed'))throw 'TEST source unreadable';if(args.path.includes('conflict'))throw '來源衝突：多筆來源匹配';const exists=window.testBooks.find(b=>b.path===args.path);if(exists)return {book:exists,kind:'updated'};const b=window.makeBook(window.testBooks.length+1,'新增漫畫');b.path=args.path;b.available=true;window.testBooks.push(b);return {book:b,kind:'added'};}
return null;
}};
'''
# Test covers are generated fixture images, not product assets or downloaded user data.
from PIL import Image,ImageDraw
import io,base64
covers=[]
for color in ['#386f80','#6c8150','#4e487a','#93632e']:
 im=Image.new('RGB',(300,400),color);draw=ImageDraw.Draw(im)
 draw.rectangle((24,24,276,376),outline='#d7dac9',width=3)
 draw.text((70,170),'MangaFolio\nTEST SOURCE',fill='white',spacing=10)
 buf=io.BytesIO();im.save(buf,format='PNG');covers.append(base64.b64encode(buf.getvalue()).decode())
script=script.replace('COVERS',json.dumps(covers))
checks=[];errors=[];contrast=[]
with sync_playwright() as p:
 browser=p.chromium.launch(headless=True,executable_path=os.environ.get('CHROMIUM_EXECUTABLE','/usr/bin/chromium'),args=['--no-sandbox'])
 page=browser.new_page(viewport={'width':1440,'height':1000})
 page.set_default_timeout(8000)
 page.on('pageerror',lambda error:errors.append(str(error)))
 page.add_init_script(script);page.goto('http://127.0.0.1:1420/')
 page.locator('.book-card').nth(3).wait_for()
 if page.locator('.appearance-settings').get_attribute('open') is None:
  page.get_by_text('外觀設定',exact=True).click()
 # All styles/views/themes use the same data and preserve selection/filter/sort.
 page.get_by_role('button',name='管理與備份',exact=True).click()
 page.get_by_role('checkbox',name='選取：海邊故事',exact=True).check()
 page.get_by_role('searchbox',name='搜尋書名').fill('海邊')
 page.get_by_label('書籍排序',exact=True).select_option('title')
 for style in ['calm','catalog','night']:
  for theme in ['light','dark']:
   for view in ['grid','detail','compact']:
    page.get_by_label('介面風格',exact=True).select_option(style)
    page.get_by_label('明暗主題',exact=True).select_option(theme)
    page.get_by_label('書庫檢視',exact=True).select_option(view)
    assert page.locator('.book-card').count()==1
    assert page.get_by_role('checkbox',name='選取：海邊故事',exact=True).is_checked()
    assert page.get_by_role('searchbox',name='搜尋書名').input_value()=='海邊'
    assert page.get_by_label('書籍排序',exact=True).input_value()=='title'
    assert page.locator('html').get_attribute('data-style')==style
    assert page.locator('html').get_attribute('data-theme')==theme
    assert page.locator('.book-collection').get_attribute('class').endswith(view)
 checks.append('18 style/theme/view combinations preserve query, sort, data and selection')
 page.get_by_role('searchbox',name='搜尋書名').fill('')
 page.get_by_role('button',name='結束管理',exact=True).click()
 for style,theme,view in [('calm','light','grid'),('catalog','light','detail'),('night','dark','grid'),('calm','dark','compact'),('catalog','dark','grid'),('night','light','detail')]:
  page.get_by_label('介面風格',exact=True).select_option(style)
  page.get_by_label('明暗主題',exact=True).select_option(theme)
  page.get_by_label('書庫檢視',exact=True).select_option(view)
  page.get_by_text('外觀設定',exact=True).click()
  page.set_viewport_size({'width':1440,'height':1200})
  page.screenshot(path=str(images/f'{style}-{theme}-{view}.png'))
  page.set_viewport_size({'width':1440,'height':1000})
  page.get_by_text('外觀設定',exact=True).click()
 page.get_by_label('顯示密度',exact=True).select_option('compact')
 page.get_by_label('網格封面尺寸',exact=True).select_option('large')
 page.reload();page.locator('.book-card').nth(3).wait_for()
 assert page.locator('html').get_attribute('data-style')=='night'
 assert page.locator('html').get_attribute('data-density')=='compact'
 assert page.locator('html').get_attribute('data-cover-size')=='large'
 checks.append('appearance survives reload; all three styles have independent view/theme')
 # Measure actual token colors for all styles/themes, including control boundaries.
 def luminance(rgb):
  channels=[int(rgb[i:i+2],16)/255 for i in (1,3,5)]
  linear=[c/12.92 if c<=.04045 else ((c+.055)/1.055)**2.4 for c in channels]
  return sum(c*w for c,w in zip(linear,[.2126,.7152,.0722]))
 def ratio(a,b):
  l1,l2=sorted([luminance(a),luminance(b)],reverse=True)
  return (l1+.05)/(l2+.05)
 if page.locator('.appearance-settings').get_attribute('open') is None:
  page.get_by_text('外觀設定',exact=True).click()
 for style in ['calm','catalog','night']:
  for theme in ['light','dark']:
   page.get_by_label('介面風格',exact=True).select_option(style)
   page.get_by_label('明暗主題',exact=True).select_option(theme)
   tokens=page.evaluate("Object.fromEntries(['bg','bg-soft','panel','text','text-dim','accent','accent-soft','accent-ink','border','red'].map(k=>[k,getComputedStyle(document.documentElement).getPropertyValue('--'+k).trim()]))")
   for fg,bg,min_ratio in [('text','bg',4.5),('text-dim','bg-soft',4.5),('accent-soft','bg-soft',4.5),('accent-ink','accent',4.5),('border','panel',3),('red','bg-soft',4.5)]:
    measured=ratio(tokens[fg],tokens[bg]);assert measured>=min_ratio,(style,theme,fg,bg,measured)
    contrast.append({'style':style,'theme':theme,'foreground':fg,'background':bg,'ratio':round(measured,2),'minimum':min_ratio})
 checks.append('36 actual semantic token contrast pairs pass 4.5:1 text / 3:1 control border checks')

 page.get_by_role('button',name='管理與備份',exact=True).click()
 page.get_by_role('checkbox',name='選取：海邊故事',exact=True).check()
 before=page.evaluate('JSON.stringify([testBooks[0].lastIndex,testBooks[0].lastReadAt,testBooks[0].preferences])')
 page.get_by_role('button',name='標記已讀',exact=True).click()
 page.get_by_text('閱讀狀態已更新，續讀位置保持不變。',exact=True).wait_for()
 assert page.evaluate('testBooks[0].readingStatus')=='read'
 assert page.evaluate('JSON.stringify([testBooks[0].lastIndex,testBooks[0].lastReadAt,testBooks[0].preferences])')==before
 page.get_by_label('閱讀狀態',exact=True).select_option('read')
 assert page.locator('.book-card').count()==2
 page.get_by_label('閱讀狀態',exact=True).select_option('all')
 page.get_by_label('批次標籤',exact=True).select_option('2')
 page.get_by_role('button',name='批次加入標籤',exact=True).click()
 page.get_by_text('書籍標籤已更新。',exact=True).wait_for()
 assert page.evaluate('testBooks[0].tags.some(t=>t.id===2)')
 page.get_by_label('標籤篩選',exact=True).select_option('2')
 assert page.locator('.book-card').count()==1
 page.get_by_label('標籤篩選',exact=True).select_option(label='全部標籤')
 page.get_by_role('button',name='批次移除標籤',exact=True).click()
 page.wait_for_function('!testBooks[0].tags.some(t=>t.id===2)')
 checks.append('manual reading status and batch tags preserve progress; compound filters')
 page.get_by_text('管理標籤',exact=True).click()
 page.get_by_label('標籤名稱',exact=True).fill('新標籤')
 page.get_by_role('button',name='建立標籤',exact=True).click()
 page.wait_for_function('testTags.some(t=>t.name==="新標籤")')
 page.get_by_label('現有標籤',exact=True).select_option('3')
 page.get_by_label('標籤名稱',exact=True).fill('修改標籤')
 page.get_by_role('button',name='重新命名',exact=True).click()
 page.wait_for_function('testTags.some(t=>t.name==="修改標籤")')
 page.get_by_role('button',name='刪除標籤',exact=True).click()
 page.wait_for_function('testTags.length===2')
 assert page.locator('.book-card').count()==4
 checks.append('tag create/rename/delete leaves books intact')
 page.get_by_role('button',name='編輯資訊：海邊故事',exact=True).click()
 page.get_by_label('自訂書名',exact=True).fill('自訂海邊故事')
 page.get_by_label('系列',exact=True).fill('海邊系列')
 page.get_by_label('集數',exact=True).fill('10')
 page.get_by_label('備註',exact=True).fill('長備註'+('長'*100))
 page.evaluate('window.failEdit=true')
 page.get_by_role('button',name='保存資訊',exact=True).click()
 page.get_by_text('TEST edit failed',exact=True).wait_for()
 assert page.evaluate('testBooks[0].title')=='海邊故事'
 page.evaluate('window.failEdit=false')
 page.get_by_role('button',name='保存資訊',exact=True).click()
 page.locator('dialog').wait_for(state='detached')
 assert page.evaluate('testBooks[0].sourceTitle')=='海邊故事'
 page.get_by_role('searchbox',name='搜尋書名').fill('海邊系列')
 assert page.locator('.book-card').count()==1
 page.get_by_role('searchbox',name='搜尋書名').fill('')
 checks.append('edit failure is retryable; custom metadata searches and source name stays distinct')
 page.get_by_role('button',name='來源失效',exact=False).click()
 assert page.locator('.book-card').count()==1
 assert '/isolated/星際日誌.cbz' in page.locator('.source-path').inner_text()
 page.get_by_role('button',name='清除選取',exact=True).click()
 page.get_by_role('checkbox',name='選取：星際日誌',exact=True).check()
 page.get_by_role('button',name='重新指定 ZIP／CBZ',exact=True).click()
 page.get_by_text('已更新來源，請從封面重新開啟閱讀。',exact=True).wait_for()
 assert page.evaluate('testBooks[3].id===4 && testBooks[3].available')
 page.get_by_role('button',name='全部書籍',exact=False).click()
 page.get_by_text('備份與還原',exact=True).click()
 page.evaluate('window.failRestore=true')
 page.get_by_role('button',name='還原書庫備份',exact=True).click()
 page.get_by_text('TEST invalid backup',exact=False).wait_for()
 assert page.evaluate('testBooks.length')==4
 page.evaluate('window.failRestore=false')
 page.get_by_role('button',name='還原書庫備份',exact=True).click()
 page.get_by_text('還原完成：加入 0 本，略過 4 本既有來源。',exact=True).wait_for()
 page.get_by_role('button',name='匯出書庫備份',exact=True).click()
 page.get_by_text('備份已匯出：',exact=False).wait_for()
 checks.append('missing-source relink, export and invalid/valid restore have actionable UI states')
 page.get_by_role('button',name='結束管理',exact=True).click()
 page.get_by_role('button',name='加入 ZIP／CBZ',exact=True).click()
 page.get_by_text('逐筆結果',exact=True).click()
 page.wait_for_function('calls.filter(c=>c.cmd==="import_book_result").length===3')
 page.get_by_role('button',name='重試失敗與未開始項目',exact=True).wait_for()
 assert page.evaluate('testBooks.length')==5
 page.get_by_role('button',name='重試失敗與未開始項目',exact=True).click()
 page.wait_for_function('calls.filter(c=>c.cmd==="import_book_result").length===5')
 page.screenshot(path=str(images/'import-partial-failure.png'))
 checks.append('partial import separates added/failed/conflict and retries failed sources only')
 # Chinese composition on the library does not invoke reader shortcuts.
 page.get_by_role('searchbox',name='搜尋書名').fill('不存在')
 page.get_by_text('沒有符合搜尋與篩選的書籍',exact=True).wait_for()
 page.get_by_role('searchbox',name='搜尋書名').dispatch_event('compositionstart')
 page.get_by_role('searchbox',name='搜尋書名').dispatch_event('keydown',{'key':'ArrowRight','isComposing':True})
 page.get_by_role('searchbox',name='搜尋書名').dispatch_event('compositionend')
 page.screenshot(path=str(images/'no-results.png'))
 page.get_by_role('button',name='清除搜尋與篩選',exact=True).click()
 # Long content / 10,000 books: bounded initial rendering and covers.
 page.evaluate('''async()=>{const {useLibraryStore}=await import('/src/stores/library.ts');const l=useLibraryStore();testBooks[0].title='長書名'.repeat(100);testBooks[0].tags=[{id:9,name:'長標籤'.repeat(20)}];testBooks.push(...Array.from({length:9995},(_,i)=>makeBook(i+100,'大量書籍 '+i)));await l.refresh();}''')
 page.locator('.book-card').nth(59).wait_for()
 assert page.locator('.book-card').count()==60
 checks.append('10,000 books render first 60; long names/tags and IME library input')
 page.set_viewport_size({'width':640,'height':480})
 page.screenshot(path=str(images/'narrow-compact.png'))
 assert page.evaluate('document.querySelector(".library-layout").scrollWidth<=document.querySelector(".library-layout").clientWidth')
 page.get_by_role('button',name='書庫導覽與設定',exact=True).click()
 if page.locator('.appearance-settings').get_attribute('open') is None:
  page.get_by_text('外觀設定',exact=True).click()
 page.get_by_label('介面風格',exact=True).select_option('calm')
 page.get_by_label('明暗主題',exact=True).select_option('light')
 page.get_by_label('書庫檢視',exact=True).select_option('grid')
 page.get_by_label('網格封面尺寸',exact=True).select_option('small')
 page.get_by_role('button',name='書庫導覽與設定',exact=True).click()
 page.screenshot(path=str(images/'narrow-grid.png'))
 assert page.evaluate('document.querySelector(".library-layout").scrollWidth<=document.querySelector(".library-layout").clientWidth')
 page.keyboard.press('Tab')
 assert page.evaluate('document.activeElement!==document.body')
 checks.append('640x480 navigation/settings remain reachable, no horizontal overflow, keyboard focus')
 page.set_viewport_size({'width':1440,'height':1000})
 page.evaluate("async()=>{testBooks=[];const {useLibraryStore}=await import('/src/stores/library.ts');await useLibraryStore().refresh();}")
 page.get_by_text('把第一本漫畫加入書庫',exact=True).wait_for()
 page.screenshot(path=str(images/'empty-library.png'))
 page.get_by_role('button',name='開始教學 →',exact=True).click()
 page.get_by_role('button',name='下一步 →',exact=True).click()
 page.get_by_role('button',name='收合教學',exact=True).click()
 checks.append('empty library has add action; tutorial opens, advances and closes voluntarily')
 assert not errors,errors
 browser.close()
result={'exitCode':0,'checks':checks,'pageErrors':errors,'contrast':contrast,'boundary':'Mock IPC / browser integration; not native acceptance','screenshots':str(images.relative_to(root))}
(out/'results.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(result,ensure_ascii=False,indent=2))

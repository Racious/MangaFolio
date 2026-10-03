"""Real Vue DOM regressions for T3; isolated in-memory IPC, not native acceptance."""
from pathlib import Path
import json, os
from playwright.sync_api import sync_playwright
root = Path(__file__).resolve().parents[1]
out = root / 'docs/third-phase-review-fix-results/browser-scroll'
out.mkdir(parents=True, exist_ok=True)
fixture = {'__file__': str(root / 'scripts/validate-next-phase.py')}
exec(compile((root / 'scripts/validate-next-phase.py').read_text().split('checks=[];errors=[];contrast=[]')[0], fixture['__file__'], 'exec'), fixture)
script = fixture['script'] + r'''
window.testBooks=Array.from({length:120},(_,i)=>Object.assign(makeBook(i+1,'漫畫 '+String(i+1).padStart(3,'0')),{series:'長篇漫畫',volume:String(i+1),available:true,lastReadAt:null,lastIndex:0,readingStatus:'unread'}));
const baseInvoke=__TAURI_INTERNALS__.invoke;
__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 if(cmd==='list_library'){await new Promise(r=>setTimeout(r,180));return structuredClone(testBooks);}
 if(cmd==='open_library_book'){
  const b=testBooks.find(b=>b.id===args.id);return {bookId:b.id,sessionId:Date.now(),favorite:b.favorite,title:b.title,pageCount:120,pages:Array.from({length:120},(_,i)=>String(i+1).padStart(3,'0')+'.png'),startIndex:0,preferences:b.preferences};
 }
 if(cmd==='render_page')return Uint8Array.from(atob(COVER),c=>c.charCodeAt(0)).buffer;
 return baseInvoke(cmd,args);
};
'''.replace('COVER', json.dumps(fixture['covers'][0]))
checks=[]; errors=[]
with sync_playwright() as p:
 browser=p.chromium.launch(headless=True, executable_path=os.environ.get('CHROMIUM_EXECUTABLE','/usr/bin/chromium'),args=['--no-sandbox'])
 page=browser.new_page(viewport={'width':1200,'height':800});page.on('pageerror',lambda e:errors.append(str(e)));page.add_init_script(script);page.goto('http://127.0.0.1:1420/')
 page.locator('.book-card').nth(59).wait_for()
 page.get_by_label('書籍排序',exact=True).select_option('title')
 def owner(): return page.locator('.library-layout' if page.viewport_size['width'] <= 700 else '.library-content')
 def position(card): return card.evaluate('(e)=>e.getBoundingClientRect().top')-owner().evaluate('(e)=>e.getBoundingClientRect().top')
 def expand():
  page.get_by_role('button',name='顯示更多書籍',exact=True).click();page.wait_for_function('document.querySelectorAll(".book-card").length===120');page.wait_for_timeout(120)
 def roundtrip(label):
  card=page.locator('[data-book-id="80"]');card.scroll_into_view_if_needed();page.wait_for_timeout(150)
  # Align its row near the top so the anchor is unambiguous, without an automatic click scroll.
  owner().evaluate('(e)=>{const c=e.querySelector("[data-book-id=\\"80\\"]");e.scrollTop+=c.getBoundingClientRect().top-e.getBoundingClientRect().top-20;}')
  page.wait_for_timeout(150);before=owner().evaluate('(e)=>e.scrollTop');offset=position(card)
  card.locator('.cover').click();page.locator('.immersive-reader').wait_for();page.keyboard.press('Escape');page.get_by_role('button',name='← 書庫',exact=True).click()
  page.wait_for_function('document.querySelectorAll(".book-card").length===120');page.wait_for_timeout(450)
  after=owner().evaluate('(e)=>e.scrollTop');new_offset=position(page.locator('[data-book-id="80"]'))
  assert abs(before-after)<3,(label,before,after)
  assert abs(offset-new_offset)<3,(label,offset,new_offset)
  checks.append({'case':label,'visibleBooks':120,'before':before,'after':after,'anchorOffset':new_offset})
  page.screenshot(path=str(out/(label+'.png')))
 expand();roundtrip('desktop-expanded-book-80')
 page.set_viewport_size({'width':640,'height':480});page.wait_for_timeout(250);roundtrip('narrow-outer-scroller-book-80')
 old_anchor=page.evaluate('''async()=>{const {useLibraryStore}=await import('/src/stores/library.ts');return useLibraryStore().scrollAnchor;}''')
 page.set_viewport_size({'width':1200,'height':800});page.wait_for_timeout(250)
 anchor=page.locator('[data-book-id="'+old_anchor['key'].split(':')[1]+'"]')
 assert abs(position(anchor)-old_anchor['offset'])<3
 checks.append({'case':'resize-narrow-to-desktop-preserves-anchor','anchor':old_anchor})
 # Series detail has its own expansion state and survives opening a late volume.
 page.evaluate('''async()=>{const {useLibraryStore}=await import('/src/stores/library.ts');useLibraryStore().section='series';}''')
 page.get_by_role('button',name='開啟系列：長篇漫畫',exact=True).click();page.wait_for_timeout(150);expand();roundtrip('series-detail-volume-80')
 page.get_by_role('button',name='返回書架',exact=True).click()
 # More than sixty separate series, then detail -> reader -> detail -> overview.
 page.evaluate('''async()=>{testBooks.forEach((b,i)=>b.series='系列 '+String(i+1).padStart(3,'0'));const {useLibraryStore}=await import('/src/stores/library.ts');await useLibraryStore().refresh();}''')
 page.get_by_role('button',name='顯示更多系列',exact=True).click();page.wait_for_function('document.querySelectorAll(".series-card").length===120')
 group=page.get_by_role('button',name='開啟系列：系列 080',exact=True);group.scroll_into_view_if_needed();page.wait_for_timeout(150)
 before=owner().evaluate('(e)=>e.scrollTop');group.click();page.wait_for_timeout(150)
 page.locator('[data-book-id="80"] .cover').click();page.locator('.immersive-reader').wait_for();page.keyboard.press('Escape');page.get_by_role('button',name='← 書庫',exact=True).click();page.locator('.book-card').wait_for();page.wait_for_timeout(450)
 page.get_by_role('button',name='返回書架',exact=True).click();page.wait_for_function('document.querySelectorAll(".series-card").length===120');page.wait_for_timeout(150)
 after=owner().evaluate('(e)=>e.scrollTop');assert abs(before-after)<3,(before,after)
 checks.append({'case':'expanded-series-80-detail-reader-return','visibleSeries':120,'before':before,'after':after})
 assert not errors,errors
 (out/'results.json').write_text(json.dumps({'mode':'browser mock IPC / actual Vue DOM; not native Windows','checks':checks,'pageErrors':errors},ensure_ascii=False,indent=2))
 print(json.dumps({'checks':checks,'pageErrors':errors},ensure_ascii=False))
 browser.close()

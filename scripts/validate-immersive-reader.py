"""Browser regression checks of actual Vue reader chrome with isolated in-memory IPC.
Not native desktop acceptance. Requires Vite :1420 and Python Playwright.
"""
from pathlib import Path
import json, os
from playwright.sync_api import sync_playwright
root = Path(__file__).resolve().parents[1]
out = root/os.environ.get('MANGAFOLIO_READER_RESULT_DIR','docs/next-phase-results/visual-refinement')
images = root/os.environ.get('MANGAFOLIO_READER_IMAGE_DIR','docs/images/next-phase/refinement-after')
out.mkdir(parents=True, exist_ok=True)
images.mkdir(parents=True, exist_ok=True)
# Reuse the library fixture setup, without executing its integration run.
fixture = {}
source = (root/'scripts/validate-next-phase.py').read_text()
exec(compile(source.split('checks=[];errors=[];contrast=[]')[0], str(root/'scripts/validate-next-phase.py'), 'exec'), {'__file__': str(root/'scripts/validate-next-phase.py'), **fixture}, fixture)
script = fixture['script']
script += '''
const originalInvoke=window.__TAURI_INTERNALS__.invoke;
window.__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 if(cmd==='render_page')return Uint8Array.from(atob('''+json.dumps(fixture['covers'][0])+'''),c=>c.charCodeAt(0)).buffer;
 return originalInvoke(cmd,args);
};
'''
checks=[]
errors=[]
with sync_playwright() as p:
 browser=p.chromium.launch(headless=True,executable_path=os.environ.get('CHROMIUM_EXECUTABLE','/usr/bin/chromium'),args=['--no-sandbox'])
 page=browser.new_page(viewport={'width':1440,'height':1000})
 page.on('pageerror',lambda e:errors.append(str(e)))
 page.add_init_script(script)
 page.goto('http://127.0.0.1:1420/')
 page.locator('.book-card').first.wait_for()
 page.evaluate('''async()=>{
 const {useReaderStore}=await import('/src/stores/reader.ts');
 const {useLibraryStore}=await import('/src/stores/library.ts');
 const r=useReaderStore();r.bookId=1;r.sessionId=1;r.title='隔離閱讀測試';r.pages=['1.png','2.png','3.png'];r.transition='none';
 window.readerTest=r;useLibraryStore().screen='reader';
 }''')
 page.locator('.page').first.wait_for()
 page.mouse.move(720,500)
 page.wait_for_timeout(220)
 opacity=lambda edge:page.locator('.reader-edge.'+edge+' .reader-chrome').evaluate('(e)=>getComputedStyle(e).opacity')
 assert opacity('top')=='0' and opacity('bottom')=='0'
 viewport=page.locator('.reader-wrap').bounding_box()
 assert viewport['y']==0 and viewport['height']==1000
 page.screenshot(path=str(images/'reader-hidden.png'))
 checks.append('opening defaults to hidden chrome; reader uses all 1440x1000 pixels')
 for edge,y in [('top',2),('bottom',998)]:
  page.mouse.move(720,y);page.wait_for_timeout(220)
  assert opacity(edge)=='1'
  assert page.locator('.reader-wrap').bounding_box()==viewport
  page.screenshot(path=str(images/f'reader-{edge}-hover.png'))
  page.mouse.move(720,500);page.wait_for_timeout(220)
  assert opacity(edge)=='0'
 checks.append('top/bottom edge hover reveals controls; leaving hides; viewport unchanged')
 page.keyboard.press('Escape');page.wait_for_timeout(220)
 assert opacity('top')=='1' and opacity('bottom')=='1'
 page.keyboard.press('Escape');page.wait_for_timeout(220)
 assert opacity('top')=='0' and opacity('bottom')=='0'
 page.evaluate("window.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',isComposing:true}))")
 assert not page.locator('.immersive-reader').evaluate('(e)=>e.classList.contains("pinned")')
 page.get_by_role('slider',name='閱讀頁碼',exact=True).focus()
 page.keyboard.press('Escape')
 assert not page.locator('.immersive-reader').evaluate('(e)=>e.classList.contains("pinned")')
 page.get_by_role('button',name='← 書庫',exact=True).focus();page.wait_for_timeout(220)
 assert opacity('top')=='1'
 checks.append('Escape toggles both bars; keyboard focus reveals hidden controls; IME and input focus do not toggle')
 page.locator('body').click(position={'x':720,'y':500});page.evaluate('window.readerTest.index=0');page.keyboard.press('ArrowRight');page.wait_for_function('window.readerTest.index===1')
 checks.append('closed bookmark dialog does not block reader ArrowRight navigation')
 page.locator('body').click(position={'x':720,'y':500})
 page.evaluate("window.readerTest.progressError='隔離測試：進度儲存失敗'")
 assert page.get_by_role('alert').is_visible()
 assert page.get_by_role('button',name='重試儲存',exact=True).is_visible()
 checks.append('save errors and retry stay visible while chrome hidden')
 page.evaluate("window.readerTest.progressError=null")
 page.set_viewport_size({'width':640,'height':480})
 page.mouse.move(320,200);page.wait_for_timeout(220)
 assert page.locator('.reader-wrap').bounding_box()['height']==480
 page.screenshot(path=str(images/'reader-narrow-hidden.png'))
 page.mouse.move(320,2);page.wait_for_timeout(220)
 assert opacity('top')=='1'
 assert page.evaluate('document.documentElement.scrollWidth<=innerWidth')
 page.screenshot(path=str(images/'reader-narrow-top.png'))
 checks.append('640x480 immersive canvas and reachable top controls without document overflow')
 # CSS hover cannot be the only touch entry point.
 touch=browser.new_context(viewport={'width':640,'height':480},has_touch=True,is_mobile=True)
 t=touch.new_page();t.add_init_script(script);t.goto('http://127.0.0.1:1420/')
 t.locator('.book-card').first.wait_for()
 t.evaluate('''async()=>{const {useReaderStore}=await import('/src/stores/reader.ts');const {useLibraryStore}=await import('/src/stores/library.ts');const r=useReaderStore();r.bookId=1;r.pages=['1.png'];useLibraryStore().screen='reader';}''')
 toggle=t.get_by_role('button',name='顯示或隱藏閱讀工具列',exact=True)
 assert toggle.is_visible();toggle.tap();assert toggle.get_attribute('aria-pressed')=='true';toggle.tap();assert toggle.get_attribute('aria-pressed')=='false'
 checks.append('touch control explicitly shows/hides bars without turning a page')
 assert not errors,errors
 browser.close()
result={'exitCode':0,'checks':checks,'pageErrors':errors,'boundary':'Actual Vue components / isolated browser mock IPC; not Windows native acceptance'}
(out/'immersive-reader.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(result,ensure_ascii=False,indent=2))

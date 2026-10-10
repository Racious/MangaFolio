import { test } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { nextTick } from "vue";
import { createLibraryHarness } from "./library-harness.mjs";
import { makeRenderer, compileComponent, find, textContent } from "./vue-harness.mjs";
const settle = async () => { for (let i = 0; i < 12; i++) await nextTick(); };

test("all six library layouts open an expanded tutorial, navigate topics and restore focus without changing filters", async () => {
  const harness = await createLibraryHarness(); let app;
  const previous = Object.fromEntries(["window", "document", "requestAnimationFrame", "localStorage"].map(key => [key, globalThis[key]]));
  const focus = { isConnected: true, focus() { restored++; } }; let restored = 0;
  globalThis.window = { matchMedia: () => ({ matches: false }), addEventListener() {}, removeEventListener() {} };
  globalThis.document = { activeElement: focus, querySelector: () => null };
  globalThis.requestAnimationFrame = callback => { callback(); return 0; };
  // Even users who previously dismissed the guide get its contents immediately.
  const settingsStorage = new Map([["mangafolio.library-guide.v1.hidden", "true"], ["mangafolio.library-guide.v1.step", "9"]]);
  const storageWrites = [];
  globalThis.localStorage = { getItem: key => settingsStorage.get(key) ?? null, setItem(key, value) { storageWrites.push([key, value]); settingsStorage.set(key, value); } };
  try {
    const { directory, library, stubs } = harness;
    await compileComponent(directory, "LibraryGuide", {});
    await compileComponent(directory, "TutorialDialog", { "./LibraryGuide.vue": "./LibraryGuide.mjs" });
    const replacements = Object.fromEntries(["@tauri-apps/plugin-dialog", "../api/library", "../stores/library", "../stores/reader", "../stores/appearance", "../lib/library", "../lib/series"].map(path => [path, "./stubs.mjs"]));
    for (const name of ["BookDetailsPanel", "SeriesShelf", "BookCover", "BookCard", "ContinueReading", "BookEditor", "LibraryManager", "SettingsDialog", "LibraryNavigation", "ImportPanel"]) replacements[`./${name}.vue`] = "./stubs.mjs";
    replacements["./TutorialDialog.vue"] = "./TutorialDialog.mjs";
    const View = await compileComponent(directory, "LibraryView", replacements);
    library.query = "保留搜尋"; library.filter = "favorites"; library.statusFilter = "read"; library.selectedIds = [2];
    for (const style of ["workbench", "gallery", "studio", "calm", "catalog", "night"]) {
      stubs.appearance.settings.style = style;
      const root = { children: [] }; app = makeRenderer().createApp(View); app.mount(root); await settle();
      const open = () => find(root, n => n.props?.["aria-label"] === "開啟教學");
      assert.ok(open(), style); open().props.onClick(); await settle();
      const dialog = () => find(root, n => n.type === "dialog");
      assert.equal(dialog().modal, true); assert.match(textContent(dialog()), /加入漫畫與影片/);
      assert.ok(find(root, n => n.type === "h3" && textContent(n) === "加入漫畫與影片"));
      assert.equal(find(root, n => n.type === "ol").children.filter(n => n.type === "li").length, 3);
      const button = label => find(root, n => n.type === "button" && textContent(n).trim() === label);
      button("下一步 →").props.onClick(); await settle();
      assert.ok(find(root, n => n.type === "h3" && textContent(n) === "從封面開始閱讀"));
      button("7. 閱讀狀態與標籤").props.onClick(); await settle();
      const tagGuide = textContent(dialog());
      assert.ok(tagGuide.indexOf('在「標籤名稱」輸入') < tagGuide.indexOf('按「建立標籤」'));
      assert.ok(tagGuide.indexOf('按「建立標籤」') < tagGuide.indexOf('按「批次加入標籤」'));
      assert.match(tagGuide, /建立標籤後仍需套用到作品/);
      for (const name of ["create-tag", "assign-tag"]) {
        const path = `/guide/${name}.jpg`;
        assert.ok(find(root, n => n.type === "img" && n.props.src === path && n.props.alt));
        const bytes = await readFile(new URL(`../public${path}`, import.meta.url));
        assert.equal(bytes.readUInt16BE(0), 0xffd8, "screenshot must be a real JPEG asset");
      }
      button("9. 找到原始檔案與安全移除").props.onClick(); await settle();
      assert.match(textContent(dialog()), /不會刪除原始檔案/);
      assert.ok(find(root, n => n.type === "img" && n.props.src === "/guide/source-location.jpg"));
      assert.equal((await readFile(new URL("../public/guide/source-location.jpg", import.meta.url))).readUInt16BE(0), 0xffd8);
      dialog().props.onCancel({ preventDefault() {} }); await settle(); assert.equal(dialog(), undefined);
      open().props.onClick(); await settle();
      assert.ok(find(root, n => n.type === "h3" && textContent(n) === "加入漫畫與影片"));
      button("12. 選擇自己的介面").props.onClick(); await settle();
      button("完成教學").props.onClick(); await settle(); assert.equal(dialog(), undefined);
      assert.equal(library.query, "保留搜尋"); assert.equal(library.filter, "favorites");
      assert.equal(library.statusFilter, "read"); assert.deepEqual(library.selectedIds, [2]);
      app.unmount(); app = null;
    }
    assert.equal(restored, 12);
    assert.deepEqual(storageWrites, [], "standalone navigation must not overwrite embedded guide preferences");
    assert.equal(settingsStorage.get("mangafolio.library-guide.v1.step"), "9");
    assert.equal(settingsStorage.get("mangafolio.library-guide.v1.hidden"), "true");
  } finally { app?.unmount(); for (const [key, value] of Object.entries(previous)) globalThis[key] = value; await harness.cleanup(); }
});

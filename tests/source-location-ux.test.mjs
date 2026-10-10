import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, writeFile, rm } from "node:fs/promises";
import { pathToFileURL } from "node:url";
import { nextTick } from "vue";
import { makeRenderer, compileComponent, find, textContent } from "./vue-harness.mjs";
const settle = async () => { for (let i = 0; i < 12; i++) await nextTick(); };
test("source button works for drawer and modal, guards pending and retries missing sources", async () => {
  const directory = await mkdtemp(new URL("../.source-test-", import.meta.url));
  const previousWindow = globalThis.window, previousDocument = globalThis.document;
  globalThis.window = { matchMedia: () => ({ matches: true, addEventListener() {}, removeEventListener() {} }), addEventListener() {}, removeEventListener() {} };
  globalThis.document = { activeElement: null, querySelector: () => null };
  let app;
  try {
    await writeFile(`${directory}/stubs.mjs`, `
      import { reactive } from 'vue';
      export const library = reactive({ books: [], favoritePending: new Set(), managing: false });
      export const useLibraryStore = () => library, useReaderStore = () => ({});
      export { progressPercent, statusLabels, videoStatusLabels, displayPath } from '../src/lib/library.ts';
      export const nextVolume = () => ({ reason: '' }), pickCover = async () => null, replaceCover = async () => {};
      export const calls = [], options = { resolve: null, fail: true };
      export const showSourceLocation = id => { calls.push(id); return new Promise((resolve, reject) => options.resolve = () => options.fail ? reject(Error('來源失效，請重新連結')) : resolve()); };
      export default { render: () => null };
    `);
    const stubs = await import(pathToFileURL(`${directory}/stubs.mjs`));
    const Panel = await compileComponent(directory, "BookDetailsPanel", Object.fromEntries(["../stores/library", "../stores/reader", "../api/media", "../lib/library", "../lib/series", "./BookCover.vue", "./BookmarksPanel.vue"].map(path => [path, "./stubs.mjs"])));
    for (const presentation of ["drawer", "modal"]) {
      const root = { children: [] };
      // An offline source still allows a fresh backend check and a useful error.
      const book = { id: 4, title: "作品", path: "D:/作品.cbz", format: "cbz", available: false, tags: [], readingStatus: "unread" };
      app = makeRenderer().createApp(Panel, { book, presentation }); app.mount(root); await settle();
      const button = () => find(root, n => n.props?.["aria-label"] === "在檔案總管中顯示");
      const opening = button().props.onClick(); await settle();
      assert.equal(button().props.disabled, true); assert.equal(stubs.library.managing, true);
      await button().props.onClick(); assert.equal(stubs.calls.length, presentation === "drawer" ? 1 : 3);
      stubs.options.resolve(); await opening; await settle();
      assert.match(textContent(find(root, n => n.props?.role === "alert")), /重新連結/);
      assert.equal(button().props.disabled, false);
      stubs.options.fail = false;
      const retry = button().props.onClick(); stubs.options.resolve(); await retry; await settle();
      assert.equal(find(root, n => n.props?.role === "alert"), undefined);
      assert.equal(find(root, n => n.type === "dialog").open, true);
      app.unmount(); app = null; stubs.options.fail = true;
    }
    assert.deepEqual(stubs.calls, [4, 4, 4, 4]);
  } finally { app?.unmount(); globalThis.window = previousWindow; globalThis.document = previousDocument; await rm(directory, { recursive: true, force: true }); }
});

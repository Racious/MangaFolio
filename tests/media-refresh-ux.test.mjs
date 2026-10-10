import { test } from "node:test";
import assert from "node:assert/strict";
import { nextTick } from "vue";
import { createLibraryHarness } from "./library-harness.mjs";
import { makeRenderer, compileComponent, find } from "./vue-harness.mjs";

test("external video refresh keeps real book cards and covers mounted while metadata updates", async t => {
  t.mock.timers.enable({ apis: ["setTimeout"] });
  const harness = await createLibraryHarness(); let app;
  const previous = Object.fromEntries(["window", "document", "requestAnimationFrame", "IntersectionObserver"].map(key => [key, globalThis[key]]));
  globalThis.window = { matchMedia: () => ({ matches: false }), addEventListener() {}, removeEventListener() {} };
  globalThis.document = { activeElement: null, querySelector: () => null };
  globalThis.requestAnimationFrame = callback => { callback(); return 0; };
  let observed = 0;
  globalThis.IntersectionObserver = class {
    constructor(callback) { this.callback = callback; }
    observe() { observed++; this.callback([{ isIntersecting: true }]); }
    disconnect() {}
  };
  const settle = async () => { for (let i = 0; i < 12; i++) await nextTick(); };
  try {
    const { directory, library, ipc, stubs } = harness;
    const books = [1, 2].map(id => ({ id, title: `影片${id}`, path: `D:/${id}.mp4`, format: "video", available: true, favorite: false, tags: [], series: "", volume: "", notes: "", readingStatus: "unread", lastReadAt: null, pageCount: 1, lastIndex: 0 }));
    library.books = books;
    await compileComponent(directory, "BookCard", { "../api/covers": "./stubs.mjs", "../lib/library": "./stubs.mjs" });
    const replacements = Object.fromEntries(["@tauri-apps/plugin-dialog", "../api/library", "../stores/library", "../stores/reader", "../stores/appearance", "../lib/library", "../lib/series"].map(path => [path, "./stubs.mjs"]));
    for (const name of ["BookDetailsPanel", "SeriesShelf", "BookCover", "ContinueReading", "BookEditor", "LibraryManager", "SettingsDialog", "TutorialDialog", "LibraryNavigation", "ImportPanel"]) replacements[`./${name}.vue`] = "./stubs.mjs";
    replacements["./BookCard.vue"] = "./BookCard.mjs";
    const View = await compileComponent(directory, "LibraryView", replacements);
    const root = { children: [] }; app = makeRenderer().createApp(View); app.mount(root); await settle();
    const collection = () => find(root, node => node.type === "section" && node.props?.class?.includes("book-collection"));
    const initialCollection = collection(); assert.ok(initialCollection);
    assert.equal(observed, 2); assert.equal(stubs.calls.covers, 2);
    const revision = library.revision;
    let completeList;
    ipc.options.list = () => new Promise(resolve => { completeList = resolve; });
    const opening = library.openMedia(1); await settle();
    assert.equal(typeof completeList, "function");
    assert.equal(library.managing, true); assert.equal(library.loading, false);
    assert.equal(collection(), initialCollection); assert.equal(stubs.calls.covers, 2);
    const updated = books.map(book => ({ ...book, lastReadAt: book.id === 1 ? 123 : null }));
    completeList(updated); assert.equal(await opening, true); await settle();
    assert.equal(library.books[0].lastReadAt, 123); assert.equal(library.revision, revision);
    assert.equal(collection(), initialCollection); assert.equal(observed, 2); assert.equal(stubs.calls.covers, 2);
    // Explicit full refresh still invalidates source/cover state as before.
    ipc.options.list = async () => updated;
    await library.refresh(); await settle();
    assert.equal(library.revision, revision + 1); assert.equal(stubs.calls.covers, 4);
  } finally {
    app?.unmount(); for (const [key, value] of Object.entries(previous)) globalThis[key] = value;
    await harness.cleanup();
  }
});

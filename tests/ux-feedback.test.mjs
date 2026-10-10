import { test } from "node:test";
import assert from "node:assert/strict";
import { readFile, writeFile, mkdtemp, rm } from "node:fs/promises";
import { pathToFileURL } from "node:url";
import { parse, compileScript } from "@vue/compiler-sfc";
import ts from "typescript";
import { createRenderer, reactive, nextTick, h } from "vue";
import { createPinia, setActivePinia } from "pinia";

function makeRenderer() {
  return createRenderer({
    createElement: type => ({ type, tagName: type.toUpperCase(), props: {}, children: [], parent: null, scrollTop: 0,
      addEventListener() {}, removeEventListener() {}, get options() { return this.children; },
      querySelectorAll() { return []; }, getBoundingClientRect() { return { top: 0, bottom: 800 }; },
      close() { this.open = false; }, show() { this.open = true; this.modal = false; },
      showModal() { this.open = true; this.modal = true; } }),
    createText: text => ({ type: "text", text }), createComment: () => ({ type: "comment" }),
    setText: (node, text) => { node.text = text; },
    setElementText: (node, text) => { node.text = text; node.children = []; },
    parentNode: node => node.parent, nextSibling: node => {
      const siblings = node.parent?.children ?? [];
      return siblings[siblings.indexOf(node) + 1] ?? null;
    },
    patchProp: (node, key, _old, value) => { node.props[key] = value; },
    insert: (node, parent, anchor) => {
      if (node.parent) node.parent.children = node.parent.children.filter(child => child !== node);
      node.parent = parent;
      const index = parent.children.indexOf(anchor);
      if (index < 0) parent.children.push(node); else parent.children.splice(index, 0, node);
    },
    remove: node => { node.parent.children = node.parent.children.filter(child => child !== node); },
  });
}
const find = (node, match) => match(node) ? node : node.children?.map(child => find(child, match)).find(Boolean);
const textContent = node => (node.text ?? "") + (node.children?.map(textContent).join("") ?? "");
async function compileComponent(directory, name, replacements) {
  const source = await readFile(new URL(`../src/components/${name}.vue`, import.meta.url), "utf8");
  const { descriptor } = parse(source);
  const compiled = compileScript(descriptor, { id: name, inlineTemplate: true });
  let js = ts.transpileModule(compiled.content, {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext },
  }).outputText;
  for (const [from, to] of Object.entries(replacements)) js = js.replaceAll(`"${from}"`, `"${to}"`);
  await writeFile(`${directory}/${name}.mjs`, js);
  return (await import(pathToFileURL(`${directory}/${name}.mjs`))).default;
}

// Compile real SFCs and exercise rendered event handlers without desktop IPC.
test("reader reveals at 80px, delays hiding 400ms, and preserves chrome and pinned controls", async context => {
  const directory = await mkdtemp(new URL("../.ux-test-", import.meta.url));
  const oldWindow = globalThis.window, oldDocument = globalThis.document, oldElement = globalThis.Element;
  const keys = new Map();
  globalThis.window = { addEventListener: (name, fn) => keys.set(name, fn), removeEventListener: name => keys.delete(name) };
  globalThis.document = { querySelector: () => null }; globalThis.Element = class {};
  context.mock.timers.enable({ apis: ["setTimeout"] });
  let app;
  try {
    await writeFile(`${directory}/stubs.mjs`, `
      import { reactive } from 'vue';
      export const settings = reactive({ readerPinned: false });
      export const useAppearanceStore = () => ({ settings });
    `);
    const Reader = await compileComponent(directory, "ImmersiveReader", { "../stores/appearance": "./stubs.mjs" });
    let clicks = 0, wheels = 0;
    const root = { children: [] };
    app = makeRenderer().createApp({ render: () => h(Reader, {}, {
      default: () => h("div", { class: "comic", onClick: () => clicks++, onWheel: () => wheels++ }),
      top: () => h("button", "top"), bottom: () => h("button", "bottom"), alerts: () => h("p", "error stays"),
    }) });
    app.mount(root);
    const section = find(root, n => n.type === "section");
    const edge = name => find(root, n => n.props?.class?.includes(`reader-edge ${name}`));
    const shown = name => edge(name).props.class.includes("revealed");
    const move = async y => { section.props.onPointermove({ currentTarget: section, clientY: y, pointerType: "mouse" }); await nextTick(); };
    await move(80); assert.equal(shown("top"), true);
    await move(81); context.mock.timers.tick(399); await nextTick(); assert.equal(shown("top"), true);
    await move(70); context.mock.timers.tick(1); await nextTick(); assert.equal(shown("top"), true, "re-entry cancels hide");
    await move(81); context.mock.timers.tick(400); await nextTick(); assert.equal(shown("top"), false);
    await move(720); assert.equal(shown("bottom"), true);
    const chrome = find(edge("bottom"), n => n.props?.class === "reader-chrome");
    chrome.props.onPointerenter(); await move(600); context.mock.timers.tick(1000); await nextTick();
    assert.equal(shown("bottom"), true, "toolbar hover holds even beyond 80px");
    chrome.props.onPointerleave(); context.mock.timers.tick(400); await nextTick(); assert.equal(shown("bottom"), false);
    await move(10); section.props.onPointerleave(); context.mock.timers.tick(400); await nextTick(); assert.equal(shown("top"), false);
    const comic = find(root, n => n.props?.class === "comic"); comic.props.onClick(); comic.props.onWheel();
    assert.equal(clicks, 1); assert.equal(wheels, 1);
    assert.equal(section.props.onClick, undefined); assert.equal(section.props.onWheel, undefined);
    keys.get("keydown")({ key: "Escape", target: null }); await nextTick(); assert.ok(section.props.class.includes("pinned"));
    find(root, n => n.props?.class === "touch-controls").props.onClick(); await nextTick();
    assert.equal(section.props.class.includes("pinned"), false);
    const { settings } = await import(pathToFileURL(`${directory}/stubs.mjs`));
    settings.readerPinned = true; await nextTick(); assert.ok(section.props.class.includes("pinned"));
    assert.match(textContent(root), /error stays/);
    const source = await readFile(new URL("../src/components/ImmersiveReader.vue", import.meta.url), "utf8");
    assert.match(source, /\.reader-edge\s*\{[^}]*height: 24px;[^}]*pointer-events: none;/);
    assert.match(source, /\.reader-edge:focus-within \.reader-chrome/);
    app.unmount(); app = null; context.mock.timers.tick(1000); assert.equal(keys.size, 0);
  } finally {
    app?.unmount(); context.mock.timers.reset(); globalThis.window = oldWindow;
    globalThis.document = oldDocument; globalThis.Element = oldElement;
    await rm(directory, { recursive: true, force: true });
  }
});

test("book-card interactions agree across grid/detail/compact and both modes", async () => {
  const directory = await mkdtemp(new URL("../.ux-test-", import.meta.url));
  const previousObserver = globalThis.IntersectionObserver;
  globalThis.IntersectionObserver = class { observe() {} disconnect() {} };
  try {
    await writeFile(`${directory}/stubs.mjs`, `
      export const PhStar = () => null, PhPencilSimple = () => null, PhPlay = () => null, PhBookOpen = () => null;
      export const loadCover = async () => null;
      export const progressPercent = () => 0;
      export const statusLabels = { unread: '未讀' }, videoStatusLabels = { unread: '未看' };
      export const displayPath = value => value ?? '';
    `);
    const Card = await compileComponent(directory, "BookCard", {
      "@phosphor-icons/vue": "./stubs.mjs", "../api/covers": "./stubs.mjs", "../api/media": "./stubs.mjs", "../lib/library": "./stubs.mjs",
    });
    const renderer = makeRenderer();
    const click = async node => {
      const event = { target: node, stopPropagation() { this.stopped = true; } };
      for (let current = node; current && !event.stopped; current = current.parent) {
        event.currentTarget = current;
        current.props?.onClick?.(event);
      }
      await nextTick();
    };
    for (const view of ["grid", "detail", "compact"]) {
      for (const available of [true, false]) {
        const calls = [];
        const book = { id: 7, title: "測試書", available, tags: [], readingStatus: "unread", pageCount: 10 };
        const props = reactive({ book, view, selectable: false, selected: false, opening: false,
          favoritePending: false, onOpen: id => calls.push(["open", id]),
          onEdit: b => calls.push(["edit", b.id]), onSelect: id => calls.push(["select", id]),
          onFavorite: b => calls.push(["favorite", b.id]) });
        const root = { children: [] };
        const app = renderer.createApp({ render: () => h(Card, props) });
        app.mount(root);
        const button = cls => find(root, n => n.props?.class?.split(" ").includes(cls));
        const cover = button("cover");
        assert.equal(cover.props.disabled, !available);
        if (available) { await click(cover); assert.deepEqual(calls.pop(), ["open", 7]); }
        await click(button("card-action")); assert.deepEqual(calls.pop(), ["edit", 7]);
        await click(find(root, n => n.type === "h2")); assert.deepEqual(calls.pop(), ["edit", 7]);
        await click(button("favorite")); assert.deepEqual(calls.pop(), ["favorite", 7]);
        assert.equal(calls.length, 0);
        props.selectable = true;
        await nextTick();
        assert.equal(cover.props.disabled, false, "offline sources remain selectable");
        assert.equal(cover.props["aria-label"], "選取：測試書");
        assert.equal(cover.props["aria-pressed"], false);
        await click(cover); assert.deepEqual(calls.pop(), ["select", 7]);
        await click(button("card-action")); assert.deepEqual(calls.pop(), ["select", 7]);
        await click(button("favorite")); assert.deepEqual(calls.pop(), ["favorite", 7]);
        await click(button("edit")); assert.deepEqual(calls.pop(), ["edit", 7]);
        props.selected = true; await nextTick();
        assert.equal(cover.props["aria-label"], "取消選取：測試書");
        assert.equal(cover.props["aria-pressed"], true);
        props.selectable = false; await nextTick();
        if (available) { await click(cover); assert.deepEqual(calls.pop(), ["open", 7]); }
        assert.equal(calls.length, 0, "embedded controls never bubble into a second action");
        app.unmount();
      }
    }
  } finally {
    globalThis.IntersectionObserver = previousObserver;
    await rm(directory, { recursive: true, force: true });
  }
});

test("library shows source recovery only for confirmed missing sources and preserves other error contexts", async () => {
  const directory = await mkdtemp(new URL("../.ux-test-", import.meta.url));
  const previousWindow = globalThis.window, previousFrame = globalThis.requestAnimationFrame;
  globalThis.window = { matchMedia: () => ({ matches: true }), addEventListener() {}, removeEventListener() {} };
  globalThis.requestAnimationFrame = callback => setTimeout(callback, 0);
  let app;
  try {
    await writeFile(`${directory}/stubs.mjs`, `
      import { reactive } from 'vue';
      export { filterBooks, sortBooks } from '../src/lib/library.ts';
      export { groupSeries, seriesKey, sortVolumes } from '../src/lib/series.ts';
      export const library = reactive({ books: [], tags: [], visibleBooks: [], selectedIds: [],
        query: '保留搜尋', statusFilter: 'read', tagFilter: 2, filter: 'all', section: 'books', activeSeries: null,
        sort: 'recent', bookLimit: 60, seriesLimit: 60, favoritePending: new Set(), loading: false,
        importProgress: { results: [], completed: 0, total: 0 }, error: '', importNotice: '' });
      export const reader = reactive({ error: '壓縮檔損毀', missingBookId: null, hasBook: false });
      export const useLibraryStore = () => library, useReaderStore = () => reader;
      export const useAppearanceStore = () => ({ settings: { style:'workbench', view: 'grid' } });
      export const pickBookFiles = async () => null, pickFolder = async () => null, ask = async () => false,
        assignSeries = async () => {};
      export const PhBooks = () => null, PhStar = () => null, PhClockCounterClockwise = () => null, PhFolderNotchOpen = () => null, PhGearSix = () => null, PhPlus = () => null, PhFunnel = () => null, PhSquaresFour = () => null, PhList = () => null, PhRows = () => null, PhMagnifyingGlass = () => null, PhQuestion = () => null;
      export default { render: () => null };
    `);
    const replacements = { "@phosphor-icons/vue": "./stubs.mjs", "@tauri-apps/plugin-dialog": "./stubs.mjs",
      "../api/library": "./stubs.mjs", "../api/backend": "./stubs.mjs", "../api/media": "./stubs.mjs", "../stores/library": "./stubs.mjs",
      "../stores/reader": "./stubs.mjs", "../stores/appearance": "./stubs.mjs",
      "../lib/library": "./stubs.mjs", "../lib/series": "./stubs.mjs" };
    for (const name of ["BookDetailsPanel", "SeriesShelf", "BookCover", "ContinueReading", "BookCard", "BookEditor",
      "LibraryGuide", "LibraryManager", "TagManager", "AppearanceSettings", "ImportPanel", "SettingsDialog", "TutorialDialog", "LibraryNavigation"]) replacements[`./${name}.vue`] = "./stubs.mjs";
    const View = await compileComponent(directory, "LibraryView", replacements);
    const { library, reader } = await import(pathToFileURL(`${directory}/stubs.mjs`));
    const root = { children: [] };
    app = makeRenderer().createApp(View); app.mount(root);
    assert.ok(find(root, node => node.props?.['aria-label'] === '媒體類型'), 'narrow workbench must retain its media switch');
    const recovery = () => find(root, node => node.type === "button" && textContent(node).includes("前往來源失效／重新指定"));
    for (const error of ["壓縮檔損毀", "收藏失敗", "頁面算繪失敗"]) {
      reader.error = error; await nextTick();
      assert.equal(recovery(), undefined);
      assert.equal(library.query, "保留搜尋");
      assert.equal(library.statusFilter, "read"); assert.equal(library.tagFilter, 2);
      assert.equal(library.filter, "all");
    }
    reader.error = "來源不存在"; reader.missingBookId = 9; await nextTick();
    assert.ok(recovery());
    reader.missingBookId = null; await nextTick();
    assert.equal(recovery(), undefined);
    reader.error = ""; await nextTick();
    assert.equal(recovery(), undefined);
  } finally {
    app?.unmount();
    globalThis.window = previousWindow; globalThis.requestAnimationFrame = previousFrame;
    await rm(directory, { recursive: true, force: true });
  }
});

test("details switch without remounting, reject late covers/bookmarks, and adapt modal/Escape", async () => {
  const directory = await mkdtemp(new URL("../.ux-test-", import.meta.url));
  const previousWindow = globalThis.window, previousDocument = globalThis.document;
  const events = new Map();
  const media = { matches: true, addEventListener(_type, fn) { this.listener = fn; }, removeEventListener() {} };
  let focusRestored = false, overlay = false;
  globalThis.window = { matchMedia: () => media,
    addEventListener: (type, fn) => events.set(type, fn), removeEventListener: type => events.delete(type) };
  globalThis.document = { activeElement: { isConnected: true, focus(options) { focusRestored = options.preventScroll; } },
    querySelector: () => overlay ? {} : null };
  let app;
  try {
    await writeFile(`${directory}/stubs.mjs`, `
      import { reactive } from 'vue';
      export const library = reactive({ books: [], favoritePending: new Set() });
      export const reader = reactive({});
      export const useLibraryStore = () => library, useReaderStore = () => reader;
      export const progressPercent = () => 0, statusLabels = { unread: '未讀' }, videoStatusLabels = { unread: '未看' };
      export const displayPath = value => value;
      export const nextVolume = () => ({ reason: '沒有下一集' });
      export const covers = new Map(), bookmarks = new Map();
      export const pickCover = async () => null, replaceCover = async () => {}, showSourceLocation = async () => {};
      export const loadCover = id => new Promise(resolve => covers.set(id, resolve));
      export const listBookmarks = id => new Promise(resolve => bookmarks.set(id, resolve));
      export const saveBookmark = async () => {}, deleteBookmark = async () => {};
    `);
    const replacements = { "../api/covers": "./stubs.mjs", "../api/media": "./stubs.mjs", "../api/library": "./stubs.mjs",
      "../stores/library": "./stubs.mjs", "../stores/reader": "./stubs.mjs",
      "../lib/library": "./stubs.mjs", "../lib/series": "./stubs.mjs",
      "./BookCover.vue": "./BookCover.mjs", "./BookmarksPanel.vue": "./BookmarksPanel.mjs" };
    await compileComponent(directory, "BookCover", replacements);
    await compileComponent(directory, "BookmarksPanel", replacements);
    const Panel = await compileComponent(directory, "BookDetailsPanel", replacements);
    const ipc = await import(pathToFileURL(`${directory}/stubs.mjs`));
    const book = id => ({ id, title: `書${id}`, available: true, readingStatus: "unread", tags: [],
      path: `${id}.zip`, notes: `備註${id}`, pageCount: 10 });
    const props = reactive({ book: book(1), onClose: () => { closes++; } });
    let closes = 0;
    const root = { children: [] };
    app = makeRenderer().createApp({ render: () => h(Panel, props) });
    app.mount(root);
    const panel = find(root, node => node.type === "dialog");
    assert.equal(panel.modal, false);
    panel.scrollTop = 500;
    props.book = book(2); await nextTick();
    assert.equal(find(root, node => node.type === "dialog"), panel, "sidebar stays mounted");
    assert.equal(panel.scrollTop, 0);
    assert.match(textContent(root), /備註2/);
    assert.doesNotMatch(textContent(root), /備註1/);
    ipc.bookmarks.get(2)([{ id: 2, bookId: 2, name: "書2書籤", note: "", pageIndex: 0 }]);
    ipc.covers.get(2)("blob:book2");
    await new Promise(resolve => setTimeout(resolve, 0));
    ipc.bookmarks.get(1)([{ id: 1, bookId: 1, name: "舊書籤", note: "", pageIndex: 0 }]);
    ipc.covers.get(1)("blob:book1");
    await new Promise(resolve => setTimeout(resolve, 0));
    assert.match(textContent(root), /書2書籤/);
    assert.doesNotMatch(textContent(root), /舊書籤/);
    assert.equal(find(root, node => node.type === "img").props.src, "blob:book2");
    const escape = () => events.get("keydown")({ key: "Escape", preventDefault() {} });
    overlay = true; escape(); assert.equal(closes, 0);
    overlay = false; escape(); assert.equal(closes, 1);
    media.matches = false; media.listener(); await nextTick();
    assert.equal(panel.modal, true); assert.equal(panel.props["aria-modal"], true);
    escape(); assert.equal(closes, 1, "narrow dialog uses its native cancel event");
    panel.props.onCancel({ preventDefault() {} }); assert.equal(closes, 2);
    app.unmount(); app = null;
    assert.equal(focusRestored, true);
    assert.equal(events.size, 0);
  } finally {
    app?.unmount();
    globalThis.window = previousWindow; globalThis.document = previousDocument;
    await rm(directory, { recursive: true, force: true });
  }
});

test("manager explains relink restrictions and offers explicit pruning of hidden selections", async () => {
  const directory = await mkdtemp(new URL("../.ux-test-", import.meta.url));
  let app;
  try {
    await writeFile(`${directory}/stubs.mjs`, `
      import { reactive } from 'vue';
      export const library = reactive({ favoritePending: new Set(), tags: [], books: [], managing: false,
        importing: false, loading: false });
      export const reader = reactive({ loading: false, favoritePending: false, flushProgress: async () => {} });
      export const useLibraryStore = () => library;
      export const useReaderStore = () => reader;
      export let confirmation = ''; export const ask = async message => { confirmation = message; return false; };
      export const open = async () => null, pickFolder = async () => null, videoExtensions = ["mp4"];
      export const assignSeries = async () => {}, setReadingStatus = async () => {}, assignTag = async () => {},
        favoriteBooks = async () => {}, relinkBook = async () => {}, removeBooks = async () => {};
      export default { render: () => null };
    `);
    const Manager = await compileComponent(directory, "LibraryManager", {
      "./BackupPanel.vue": "./stubs.mjs", "@tauri-apps/plugin-dialog": "./stubs.mjs",
      "../api/library": "./stubs.mjs", "../api/backend": "./stubs.mjs", "../api/media": "./stubs.mjs",
      "../stores/library": "./stubs.mjs", "../stores/reader": "./stubs.mjs",
    });
    const props = reactive({ selectedIds: [1, 2], shownCount: 1, hiddenCount: 1,
      onRetainShown: () => { props.selectedIds = [2]; props.hiddenCount = 0; } });
    const root = { children: [] };
    app = makeRenderer().createApp({ render: () => h(Manager, props) });
    app.mount(root);
    const relink = () => find(root, node => node.type === "button" && textContent(node).includes("重新指定 ZIP"));
    const manager = find(root, node => node.type === "section" && node.props?.class === "manager");
    const live = find(root, node => node.props?.id === "relink-status");
    assert.equal(live.props.role, "status"); assert.match(textContent(live), /已選 2 本/);
    assert.equal(relink().props.disabled, true);
    assert.match(textContent(root), /一次只能選 1 本；目前已選 2 本/);
    assert.match(textContent(root), /包含 1 本未顯示的選取/);
    const retain = find(root, node => node.type === "button" && textContent(node).includes("只保留目前"));
    retain.props.onClick(); await nextTick();
    assert.deepEqual(props.selectedIds, [2]);
    assert.equal(relink().props.disabled, false);
    assert.equal(find(root, node => node.props?.id === "relink-status"), live, "live region is never recreated");
    assert.equal(textContent(live), "");
    assert.doesNotMatch(textContent(root), /未顯示的選取/);
    const remove = find(root, node => node.type === "button" && textContent(node).trim() === "移除所選");
    await remove.props.onClick(); await nextTick();
    const ipc = await import(pathToFileURL(`${directory}/stubs.mjs`));
    for (const term of ["系列／集數", "標籤關聯", "書籤／頁面筆記", "自訂書名與備註", "原始漫畫檔案與共用標籤會保留"])
      assert.ok(ipc.confirmation.includes(term));
    props.selectedIds = []; await nextTick();
    assert.doesNotMatch(textContent(root), /未顯示的選取/);
    assert.match(textContent(root), /目前已選 0 本/);
    assert.equal(relink().props.disabled, true);
    const reason = () => find(root, node => node.props?.id === "relink-reason");
    assert.ok(reason().props.class.includes("sr-only"));
    assert.equal(reason().props.role, undefined, "description is separate from the live region");
    assert.equal(textContent(live), "", "zero selection has no live announcement");
    assert.equal(relink().props["aria-describedby"], "relink-reason");
    assert.match(relink().props.title, /目前已選 0 本/);
    props.selectedIds = [1, 2]; await nextTick();
    assert.equal(find(root, node => node.props?.id === "relink-status"), live);
    assert.match(textContent(live), /已選 2 本/);
    for (const ids of [[], [1], [1, 2]]) {
      props.selectedIds = ids; await nextTick();
      const selectionNotice = textContent(live);
      for (const [store, property] of [
        [ipc.library, "loading"], [ipc.library, "importing"], [ipc.library, "managing"],
        [ipc.reader, "loading"], [ipc.reader, "favoritePending"],
      ]) {
        store[property] = true; await nextTick();
        assert.equal(manager.props["aria-busy"], true);
        assert.equal(relink().props.disabled, true);
        assert.equal(reason().props.class.includes("sr-only"), false);
        assert.equal(reason().props.role, undefined);
        assert.match(textContent(reason()), /操作進行中/);
        assert.equal(textContent(live), selectionNotice, `${property} never changes the relink announcement`);
        store[property] = false; await nextTick();
        assert.equal(manager.props["aria-busy"], false);
        assert.equal(textContent(live), selectionNotice, `${property} completion never repeats the announcement`);
      }
      ipc.library.favoritePending.add(1); await nextTick();
      assert.equal(manager.props["aria-busy"], true);
      assert.equal(textContent(live), selectionNotice);
      ipc.library.favoritePending.clear(); await nextTick();
      assert.equal(manager.props["aria-busy"], false);
      assert.equal(textContent(live), selectionNotice);
      assert.equal(find(root, node => node.props?.id === "relink-status"), live);
    }
    ipc.library.loading = true; await nextTick();
    props.selectedIds = [1, 2, 3]; await nextTick();
    assert.match(textContent(live), /已選 3 本/, "selection changes still announce during a refresh");
    ipc.library.loading = false; await nextTick();
    assert.match(textContent(live), /已選 3 本/);
  } finally {
    app?.unmount();
    await rm(directory, { recursive: true, force: true });
  }
});

test("successful relink clears the matching missing error without discarding another active book", async () => {
  const directory = await mkdtemp(new URL("../.ux-test-", import.meta.url));
  let app;
  try {
    await writeFile(`${directory}/stubs.mjs`, `
      import { reactive } from 'vue';
      export const library = reactive({ books: [{ id: 9, title: '離線書' }], tags: [], favoritePending: new Set(),
        managing: false, refresh: async () => {}, notifySuccess: () => {} });
      export const reader = reactive({ bookId: 1, missingBookId: 9, error: '來源失效',
        flushProgress: async () => {}, discardBook: () => { throw Error('must preserve the other book'); } });
      export const options = { confirm: true, path: 'isolated.cbz', fail: false };
      export const useLibraryStore = () => library, useReaderStore = () => reader;
      export const ask = async () => options.confirm, open = async () => options.path, pickFolder = open;
      export const videoExtensions = ["mp4"]; export const relinkBook = async () => { if (options.fail) throw Error('relink failed'); };
      export const assignSeries = async () => {}, setReadingStatus = async () => {}, assignTag = async () => {},
        favoriteBooks = async () => {}, removeBooks = async () => {};
      export default { render: () => null };
    `);
    const Manager = await compileComponent(directory, "LibraryManager", {
      "./BackupPanel.vue": "./stubs.mjs", "@tauri-apps/plugin-dialog": "./stubs.mjs",
      "../api/library": "./stubs.mjs", "../api/backend": "./stubs.mjs", "../api/media": "./stubs.mjs",
      "../stores/library": "./stubs.mjs", "../stores/reader": "./stubs.mjs",
    });
    const root = { children: [] };
    app = makeRenderer().createApp({ render: () => h(Manager, { selectedIds: [9], shownCount: 1, hiddenCount: 0 }) });
    app.mount(root);
    const { reader, options } = await import(pathToFileURL(`${directory}/stubs.mjs`));
    const relink = () => find(root, n => n.type === "button" && textContent(n).includes("重新指定 ZIP"));
    await relink().props.onClick();
    assert.equal(reader.error, ""); assert.equal(reader.missingBookId, null); assert.equal(reader.bookId, 1);
    for (const mode of ["other", "failed", "cancelled"]) {
      reader.error = "保留的錯誤"; reader.missingBookId = mode === "other" ? 10 : 9;
      options.fail = mode === "failed"; options.confirm = mode !== "cancelled";
      await nextTick(); await relink().props.onClick();
      assert.equal(reader.error, "保留的錯誤");
      assert.equal(reader.missingBookId, mode === "other" ? 10 : 9);
      assert.equal(reader.bookId, 1);
    }
  } finally {
    app?.unmount(); await rm(directory, { recursive: true, force: true });
  }
});

test("backup panel and real store choose/reset directories, display fallback/errors, and retain export behavior", async () => {
  const directory = await mkdtemp(new URL("../.ux-test-", import.meta.url));
  let app;
  try {
    await writeFile(`${directory}/stubs.mjs`, `
      import { reactive } from 'vue';
      export { displayPath } from '../src/lib/library.ts';
      export const notices = [];
      export const useLibraryStore = () => library;
      export const library = reactive({ favoritePending: new Set(), managing: false,
        notifySuccess: message => notices.push(message) });
      export const useReaderStore = () => ({ flushProgress: async () => {} });
      export let backendSettings = { enabled: false, retention: 5, lastSuccess: null, lastError: '',
        customDirectory: null, directory: ${JSON.stringify("\\\\?\\C:\\isolated\\backups")}, directoryWarning: '' };
      export const options = { path: ${JSON.stringify("\\\\?\\C:\\custom")}, failSelect: false, failOpen: false };
      export const getBackupSettings = async () => ({ ...backendSettings });
      export const setBackupSettings = async (enabled, retention) => ({ ...backendSettings, enabled, retention });
      export let directoryCalls = 0;
      export const setBackupDirectory = async path => {
        directoryCalls++;
        if (options.failSelect) throw Error('備份資料夾無法寫入');
        backendSettings = { ...backendSettings, customDirectory: path,
          directory: path ?? ${JSON.stringify("\\\\?\\C:\\isolated\\backups")}, directoryWarning: '' };
        return { ...backendSettings };
      };
      export const runAutomaticBackup = async () => {
        backendSettings = { ...backendSettings, directory: ${JSON.stringify("\\\\?\\C:\\isolated\\backups")},
          directoryWarning: '自訂資料夾無法使用，本次已改存預設位置。' };
        return { ...backendSettings };
      };
      export let opened = 0; export const openBackupDirectory = async () => {
        opened++; if (options.failOpen) throw Error('備份資料夾不存在或無法存取，請先建立備份');
      };
      export const save = async () => ${JSON.stringify("\\\\?\\C:\\exports\\books.json")};
      export let exported = ''; export const exportBackup = async path => { exported = path; };
      export const open = async () => null, previewBackup = async () => {}, restoreBackup = async () => {};
      export const pickFolder = async () => options.path;
    `);
    const storeSource = await readFile(new URL("../src/stores/backup.ts", import.meta.url), "utf8");
    const storeJs = ts.transpileModule(storeSource, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext } }).outputText;
    await writeFile(`${directory}/backup.mjs`, storeJs.replaceAll('"../api/library"', '"./stubs.mjs"'));
    setActivePinia(createPinia());
    const Panel = await compileComponent(directory, "BackupPanel", {
      "@tauri-apps/plugin-dialog": "./stubs.mjs", "../api/library": "./stubs.mjs",
      "../stores/library": "./stubs.mjs", "../stores/reader": "./stubs.mjs",
      "../stores/backup": "./backup.mjs", "../lib/library": "./stubs.mjs", "../api/backend": "./stubs.mjs",
    });
    const root = { children: [] };
    app = makeRenderer().createApp(Panel); app.mount(root);
    await new Promise(resolve => setTimeout(resolve, 0));
    assert.ok(textContent(root).includes("備份資料夾：C:\\isolated\\backups"));
    assert.equal(textContent(root).includes("\\\\?\\"), false);
    const button = title => find(root, node => node.type === "button" && textContent(node).trim() === title);
    await button("開啟備份資料夾").props.onClick();
    await button("匯出書庫備份").props.onClick();
    const ipc = await import(pathToFileURL(`${directory}/stubs.mjs`));
    assert.equal(ipc.opened, 1);
    assert.equal(ipc.exported, "\\\\?\\C:\\exports\\books.json", "IPC keeps the original path");
    assert.ok(ipc.notices[0].includes("C:\\exports\\books.json"));
    assert.equal(ipc.notices[0].includes("\\\\?\\"), false);
    await button("選擇備份資料夾").props.onClick(); await nextTick();
    assert.ok(textContent(root).includes("備份資料夾：C:\\custom"));
    assert.equal(button("恢復預設").props.disabled, false);
    assert.match(textContent(root), /舊備份留在原處/);
    assert.equal(ipc.directoryCalls, 1);
    await button("立即建立本機安全備份").props.onClick(); await nextTick();
    assert.match(textContent(root), /本次已改存預設位置/);
    assert.ok(textContent(root).includes("備份資料夾：C:\\isolated\\backups"));
    assert.ok(textContent(root).includes("設定的自訂資料夾：C:\\custom"));
    assert.ok(find(root, n => n.props?.role === "alert" && textContent(n).includes("本次已改存預設位置")));
    await button("恢復預設").props.onClick(); await nextTick();
    assert.equal(button("恢復預設").props.disabled, true);
    assert.doesNotMatch(textContent(root), /本次已改存預設位置/);
    ipc.options.failSelect = true;
    await button("選擇備份資料夾").props.onClick(); await nextTick();
    assert.match(textContent(root), /備份資料夾無法寫入/);
    assert.ok(textContent(root).includes("備份資料夾：C:\\isolated\\backups"), "failed validation retains settings");
    const calls = ipc.directoryCalls; ipc.options.path = null;
    await button("選擇備份資料夾").props.onClick(); assert.equal(ipc.directoryCalls, calls, "cancel does not reset");
    ipc.options.failOpen = true;
    await button("開啟備份資料夾").props.onClick(); await nextTick();
    assert.match(textContent(root), /請先建立備份/);
  } finally {
    app?.unmount();
    await rm(directory, { recursive: true, force: true });
  }
});

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFile, writeFile, mkdtemp, rm } from "node:fs/promises";
import { pathToFileURL } from "node:url";
import ts from "typescript";
import { createPinia, setActivePinia } from "pinia";

// Execute the real stores; replace only desktop IPC dependencies.
const directory = await mkdtemp(new URL("../.favorite-test-", import.meta.url));
try {
  await writeFile(
    `${directory}/ipc.mjs`,
    `export let handler; export const configure = fn => handler = fn;
    export const setFavorite = (...args) => handler(...args);
    export let listHandler; export const configureList = fn => listHandler=fn;
    export const listLibrary = async () => listHandler ? listHandler() : []; export let importHandler; export const configureImport = fn => importHandler=fn; export const importBookResult = (...args) => importHandler(...args); export const listTags = async () => [];
    export let loadHandler,saveHandler; export const configureLoad=fn=>loadHandler=fn; export const configureSave=fn=>saveHandler=fn; export const saveProgress = async (...args) => saveHandler?.(...args); export const openPath = async () => {};
    export let renderHandler; export const configureRender = fn => renderHandler=fn;
    export const openLibraryBook = async (...args) => loadHandler?.(...args); export const openBookmark = async (...args) => loadHandler?.(...args); export const renderPageUrl = async () => renderHandler ? renderHandler() : '';`,
  );
  for (const name of ["reader", "library"]) {
    let source = await readFile(
      new URL(`../src/stores/${name}.ts`, import.meta.url),
      "utf8",
    );
    let js = ts.transpileModule(source, {
      compilerOptions: {
        target: ts.ScriptTarget.ES2022,
        module: ts.ModuleKind.ESNext,
      },
    }).outputText;
    js = js
      .replaceAll('"../api/library"', '"./ipc.mjs"')
      .replaceAll('"../api/backend"', '"./ipc.mjs"')
      .replaceAll('"./reader"', '"./reader.mjs"')
      .replaceAll('"./library"', '"./library.mjs"')
      .replaceAll('"../lib/library"', '"./helpers.mjs"')
      .replaceAll('"../lib/import"', '"./imports.mjs"');
    await writeFile(`${directory}/${name}.mjs`, js);
  }
  const imports = await readFile(
    new URL("../src/lib/import.ts", import.meta.url),
    "utf8",
  );
  await writeFile(
    `${directory}/imports.mjs`,
    ts.transpileModule(imports, {
      compilerOptions: { module: ts.ModuleKind.ESNext },
    }).outputText,
  );
  const helpers = await readFile(
    new URL("../src/lib/library.ts", import.meta.url),
    "utf8",
  );
  await writeFile(
    `${directory}/helpers.mjs`,
    ts.transpileModule(helpers, {
      compilerOptions: { module: ts.ModuleKind.ESNext },
    }).outputText,
  );
  const { useLibraryStore } = await import(
    pathToFileURL(`${directory}/library.mjs`)
  );
  const { useReaderStore } = await import(
    pathToFileURL(`${directory}/reader.mjs`)
  );
  const { configure, configureImport, configureLoad, configureSave, configureList, configureRender } = await import(
    pathToFileURL(`${directory}/ipc.mjs`)
  );
  function setup() {
    configureList(undefined);
    setActivePinia(createPinia());
    const library = useLibraryStore(),
      reader = useReaderStore();
    library.books = [{ id: 1, favorite: false }];
    reader.bookId = 1;
    return { library, reader, book: library.books[0] };
  }
  test("success notices expire after five seconds and never clear errors or newer results", (t) => {
    t.mock.timers.enable({ apis: ["setTimeout"] });
    const { library } = setup();
    library.error = "重要錯誤";
    library.notifySuccess("保存成功");
    t.mock.timers.tick(4000);
    assert.equal(library.importNotice, "保存成功");
    library.notifySuccess("備份成功");
    t.mock.timers.tick(1000);
    assert.equal(library.importNotice, "備份成功");
    t.mock.timers.tick(4000);
    assert.equal(library.importNotice, "");
    assert.equal(library.error, "重要錯誤");
    library.notifySuccess("保存成功");
    library.importNotice = "新增 0 本，失敗／衝突 1 本";
    t.mock.timers.tick(5000);
    assert.match(library.importNotice, /失敗／衝突/);
    t.mock.timers.reset();
  });
  test("failed open refreshes availability and missing filter rechecks restored sources", async () => {
    const { library, reader } = setup();
    library.books = [{ id: 1, title: "Missing", available: true, tags: [] }];
    let available = false, lists = 0;
    configureList(async () => { lists++; return [{ id: 1, title: "Missing", available, tags: [] }]; });
    configureLoad(async () => { throw new Error('路徑不存在：\\\\?\\C:\\books\\missing.zip'); });
    assert.equal(await reader.openBook(1), false);
    assert.equal(library.books[0].available, false);
    assert.equal(reader.missingBookId, 1);
    assert.equal(lists, 1);
    assert.match(reader.error, /前往「來源失效」/);
    assert.equal(reader.error.includes('\\\\?\\'), false);
    await library.setFilter("missing");
    assert.deepEqual(library.visibleBooks.map(b => b.id), [1]);
    available = true;
    await library.setFilter("missing");
    assert.deepEqual(library.visibleBooks, []);
    assert.equal(lists, 3);
    configureList(undefined);
  });
  test("unrelated errors and the next successful open clear stale missing-source recovery", async () => {
    const { reader } = setup();
    configureList(async () => [{ id: 1, available: true, tags: [] }]);
    reader.missingBookId = 1;
    configureLoad(async () => { throw new Error("壓縮檔損毀，來源仍存在"); });
    assert.equal(await reader.openBook(1), false);
    assert.equal(reader.missingBookId, null);
    assert.doesNotMatch(reader.error, /前往「來源失效」/);
    reader.missingBookId = 1;
    configure(async () => { throw new Error("收藏失敗"); });
    await reader.toggleFavorite();
    assert.equal(reader.missingBookId, null);
    reader.missingBookId = 1;
    reader.pages = ["page.png"]; reader.viewportW = 640; reader.viewportH = 480;
    configureRender(async () => { throw new Error("算繪失敗"); });
    await reader.render();
    assert.match(reader.error, /算繪失敗/);
    assert.equal(reader.missingBookId, null);
    configureRender(undefined); reader.viewportW = 0; reader.viewportH = 0;
    reader.missingBookId = 1;
    configureLoad(async () => ({ bookId: 2, sessionId: 2, title: "新書", pages: ["page.png"],
      preferences: {}, startIndex: 0, favorite: false }));
    assert.equal(await reader.openBook(2), true);
    assert.equal(reader.error, "");
    assert.equal(reader.missingBookId, null);
    reader.missingBookId = 1;
    reader.discardBook();
    assert.equal(reader.missingBookId, null);
    configureList(undefined);
  });
  test("switching volume waits for save and preserves current book when save/open fails",async()=>{
    const {reader}=setup();reader.pages=["1.png","2.png"];reader.index=1;reader.title="Current";
    const before=JSON.stringify([reader.bookId,reader.pages,reader.index,reader.title]);let called=false;
    configureSave(async()=>{throw new Error("save blocked");});configureLoad(async()=>{called=true;});
    assert.equal(await reader.openBook(2),false);assert.equal(called,false);assert.equal(JSON.stringify([reader.bookId,reader.pages,reader.index,reader.title]),before);
    configureSave(async()=>{});configureLoad(async()=>{throw new Error("source conflict or offline");});
    assert.equal(await reader.openBook(2),false);assert.equal(JSON.stringify([reader.bookId,reader.pages,reader.index,reader.title]),before);
    assert.equal(await reader.openBookmark(2,5),false);assert.equal(JSON.stringify([reader.bookId,reader.pages,reader.index,reader.title]),before);reader.discardBook();configureSave(undefined);
  });
  test("card favorite and unfavorite synchronize retained reader and its next toggle", async () => {
    const { library, reader, book } = setup();
    const calls = [];
    configure(async (...args) => calls.push(args));
    await library.toggleFavorite(book);
    assert.equal(reader.favorite, true);
    assert.equal(book.favorite, true);
    await reader.toggleFavorite();
    assert.equal(reader.favorite, false);
    assert.deepEqual(calls, [
      [1, true],
      [1, false],
    ]);
    reader.favorite = true;
    await library.toggleFavorite(book);
    assert.equal(reader.favorite, false);
    assert.equal(book.favorite, false);
  });
  test("failed favorite leaves both states unchanged", async () => {
    const { library, reader, book } = setup();
    configure(async () => {
      throw new Error("IPC failed");
    });
    await library.toggleFavorite(book);
    assert.equal(book.favorite, false);
    assert.equal(reader.favorite, false);
    assert.match(library.error, /IPC failed/);
    assert.equal(library.favoritePending.size, 0);
  });
  test("switching books while favorite is pending never updates the new reader", async () => {
    const { library, reader, book } = setup();
    let complete;
    configure(
      () =>
        new Promise((resolve) => {
          complete = resolve;
        }),
    );
    const pending = library.toggleFavorite(book);
    reader.bookId = 2;
    reader.favorite = false;
    complete();
    await pending;
    assert.equal(book.favorite, true);
    assert.equal(reader.favorite, false);
    assert.equal(reader.bookId, 2);
  });
  test("store import prevents duplicate starts and honors cancellation and management lock", async () => {
    const { library } = setup();
    let complete;
    const calls = [];
    configureImport((path) => {
      calls.push(path);
      return new Promise((resolve) => {
        complete = () => resolve({ book: { id: 1 }, kind: "added" });
      });
    });
    const pending = library.add(["first", "second"]);
    // add awaits the reader progress flush before invoking the first import.
    await new Promise((resolve) => setTimeout(resolve, 0));
    await library.add(["duplicate"]);
    assert.deepEqual(calls, ["first"]);
    library.cancelImport();
    complete();
    await pending;
    assert.deepEqual(
      library.importProgress.results.map((item) => item.kind),
      ["added", "canceled"],
    );
    library.managing = true;
    await library.add(["during-management"]);
    assert.deepEqual(calls, ["first"]);
    assert.equal(library.importing, false);
  });
  test("reader final spreads retain the start index in progress snapshots", () => {
    for (const count of [1, 2, 3, 4, 5, 6, 7, 8]) {
      for (const cover of [false, true]) {
        const { reader } = setup();
        reader.pages = Array.from({ length: count }, (_, i) => `${i}.png`);
        reader.pageMode = "double";
        reader.doubleCover = cover;
        const start = count === 1 ? 0 : ((cover && count % 2 === 0) || (!cover && count % 2 === 1)) ? count - 1 : count - 2;
        reader.index = start;
        assert.equal(reader.atLast, true);
        assert.equal(Math.max(...reader.viewIndices), count - 1);
        assert.equal(reader.progressSnapshot().index, start);
        if (start > 0) {
          reader.index = start - 1;
          assert.equal(reader.atLast, false);
        }
      }
    }
  });
} finally {
  await rm(directory, { recursive: true, force: true });
}

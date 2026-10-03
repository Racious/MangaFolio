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
    export const listLibrary = async () => []; export let importHandler; export const configureImport = fn => importHandler=fn; export const importBookResult = (...args) => importHandler(...args); export const listTags = async () => [];
    export const saveProgress = async () => {}; export const openPath = async () => {};
    export const openLibraryBook = async () => {}; export const renderPageUrl = async () => '';`,
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
  const { configure, configureImport } = await import(
    pathToFileURL(`${directory}/ipc.mjs`)
  );
  function setup() {
    setActivePinia(createPinia());
    const library = useLibraryStore(),
      reader = useReaderStore();
    library.books = [{ id: 1, favorite: false }];
    reader.bookId = 1;
    return { library, reader, book: library.books[0] };
  }
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
} finally {
  await rm(directory, { recursive: true, force: true });
}

import { test } from "node:test";
import assert from "node:assert/strict";
import { importSources, type ImportProgress } from "../src/lib/import.ts";
import type { LibraryBook } from "../src/api/library.ts";
const book = { id: 1 } as LibraryBook;
test("batch import records added, updated, failed and conflicts and continues", async () => {
  const calls: string[] = [];
  const progress: ImportProgress[] = [];
  const results = await importSources(["new", "bad", "old", "duplicate"], {
    shouldCancel: () => false,
    onProgress: (p) => progress.push(p),
    importSource: async (path) => {
      calls.push(path);
      if (path === "bad") throw "無法開啟";
      if (path === "duplicate") throw "來源衝突：多筆紀錄";
      return { book, kind: path === "new" ? "added" : "updated" };
    },
  });
  assert.deepEqual(
    results.map((r) => r.kind),
    ["added", "failed", "updated", "conflict"],
  );
  assert.deepEqual(calls, ["new", "bad", "old", "duplicate"]);
  assert.equal(progress.at(-1)?.completed, 4);
  assert.equal(progress[0].results.length, 0);
  const failed = results.filter(
    (r) => r.kind === "failed" || r.kind === "conflict",
  );
  const retried = await importSources(
    failed.map((r) => r.path),
    {
      shouldCancel: () => false,
      onProgress: () => {},
      importSource: async () => ({ book, kind: "added" }),
    },
  );
  assert.deepEqual(
    retried.map((r) => r.path),
    ["bad", "duplicate"],
  );
  assert.ok(retried.every((r) => r.kind === "added"));
});
test("cancel preserves the in-flight successful item and never starts remaining sources", async () => {
  let canceled = false;
  let finish!: () => void;
  const calls: string[] = [];
  const task = importSources(["first", "second", "third"], {
    shouldCancel: () => canceled,
    onProgress: () => {},
    importSource: async (path) => {
      calls.push(path);
      await new Promise<void>((resolve) => {
        finish = resolve;
      });
      return { book, kind: "added" };
    },
  });
  canceled = true;
  finish();
  const results = await task;
  assert.deepEqual(calls, ["first"]);
  assert.deepEqual(
    results.map((r) => r.kind),
    ["added", "canceled", "canceled"],
  );
});

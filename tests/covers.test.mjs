import { test } from "node:test";
import assert from "node:assert/strict";
import { readFile, writeFile, mkdtemp, rm } from "node:fs/promises";
import { pathToFileURL } from "node:url";
import ts from "typescript";
test("cover queue bounds concurrent decoding and skips canceled offscreen jobs", async () => {
  const dir = await mkdtemp(new URL("../.cover-test-", import.meta.url));
  try {
    await writeFile(
      `${dir}/ipc.mjs`,
      "export let handler;export const configure=fn=>handler=fn;export const coverBytes=id=>handler(id);",
    );
    const source = await readFile(
      new URL("../src/api/covers.ts", import.meta.url),
      "utf8",
    );
    const js = ts
      .transpileModule(source, {
        compilerOptions: {
          target: ts.ScriptTarget.ES2022,
          module: ts.ModuleKind.ESNext,
        },
      })
      .outputText.replace('"./library"', '"./ipc.mjs"');
    await writeFile(`${dir}/covers.mjs`, js);
    const { configure } = await import(pathToFileURL(`${dir}/ipc.mjs`));
    const { loadCover } = await import(pathToFileURL(`${dir}/covers.mjs`));
    const calls = [],
      pending = [];
    let active = 0,
      maximum = 0;
    configure((id) => {
      calls.push(id);
      active++;
      maximum = Math.max(maximum, active);
      return new Promise((resolve) =>
        pending.push(() => {
          active--;
          resolve(new ArrayBuffer(1));
        }),
      );
    });
    let canceled = false;
    const jobs = [
      loadCover(1, () => false),
      loadCover(2, () => false),
      loadCover(3, () => canceled),
      loadCover(4, () => false),
    ];
    assert.deepEqual(calls, [1, 2]);
    canceled = true;
    pending.shift()();
    await new Promise((r) => setTimeout(r, 0));
    assert.deepEqual(calls, [1, 2, 4]);
    assert.equal(maximum, 2);
    while (pending.length) pending.shift()();
    const urls = await Promise.all(jobs);
    assert.equal(urls[2], null);
    for (const url of urls) if (url) URL.revokeObjectURL(url);
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
});

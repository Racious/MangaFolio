import { readFile, writeFile, mkdtemp, rm } from "node:fs/promises";
import { pathToFileURL } from "node:url";
import ts from "typescript";
import { createPinia, setActivePinia } from "pinia";

// Keep the real store and media filtering; replace only desktop IPC and reader IO.
export async function createLibraryHarness() {
  const directory = await mkdtemp(new URL("../.library-test-", import.meta.url));
  await writeFile(`${directory}/ipc.mjs`, `
    export const options = { videoError: null, list: async () => [] };
    export const calls = [];
    export const openVideo = async id => { calls.push(id); if (options.videoError) throw Error(options.videoError); };
    export const listLibrary = () => options.list(), listTags = async () => [];
    export const setFavorite = async () => {}, importBookResult = async () => {};
    export const reader = { flushProgress: async () => {}, loading: false, favoritePending: false, error: '', missingBookId: null };
    export const useReaderStore = () => reader;
  `);
  let source = await readFile(new URL("../src/stores/library.ts", import.meta.url), "utf8");
  const js = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  }).outputText
    .replaceAll('"./reader"', '"./ipc.mjs"')
    .replaceAll('"../api/library"', '"./ipc.mjs"')
    .replaceAll('"../api/media"', '"./ipc.mjs"')
    .replaceAll('"../lib/library"', '"../src/lib/library.ts"')
    .replaceAll('"../lib/import"', '"../src/lib/import.ts"');
  await writeFile(`${directory}/library.mjs`, js);
  await writeFile(`${directory}/stubs.mjs`, `
    import { reactive } from 'vue';
    export { useLibraryStore } from './library.mjs';
    export { useReaderStore } from './ipc.mjs';
    export { filterBooks, sortBooks, progressPercent, statusLabels, videoStatusLabels, displayPath } from '../src/lib/library.ts';
    export { groupSeries, seriesKey, sortVolumes, nextVolume } from '../src/lib/series.ts';
    export const appearance = reactive({ settings: { style: 'workbench', view: 'grid' } });
    export const useAppearanceStore = () => appearance;
    export const ask = async () => false, assignSeries = async () => {};
    export const pickCover = async () => null, replaceCover = async () => {};
    export const showSourceLocation = async () => {};
    export const calls = { covers: 0 };
    export const loadCover = async () => { calls.covers++; return null; };
    export default { render: () => null };
  `);
  setActivePinia(createPinia());
  const { useLibraryStore } = await import(pathToFileURL(`${directory}/library.mjs`));
  const ipc = await import(pathToFileURL(`${directory}/ipc.mjs`));
  const stubs = await import(pathToFileURL(`${directory}/stubs.mjs`));
  return { directory, library: useLibraryStore(), ipc, stubs, cleanup: () => rm(directory, { recursive: true, force: true }) };
}

import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, writeFile, rm } from "node:fs/promises";
import { pathToFileURL } from "node:url";
import { reactive, nextTick } from "vue";
import { makeRenderer, compileComponent, find, textContent } from "./vue-harness.mjs";
import { createLibraryHarness } from "./library-harness.mjs";

test("workbench media buttons restore each real store filter and stay active within favorites", async () => {
  const harness = await createLibraryHarness();
  let app;
  try {
    const { directory, library } = harness;
    const Navigation = await compileComponent(directory, "LibraryNavigation", {
      "../stores/library": "./stubs.mjs", "../lib/series": "./stubs.mjs",
    });
    const root = { children: [] };
    app = makeRenderer().createApp(Navigation, { mode: "workbench", busy: false });
    app.mount(root);
    const button = label => find(root, n => n.type === "button" && textContent(n).startsWith(label));
    await library.setFilter("favorites"); library.query = "漫畫"; library.statusFilter = "read";
    button("影片").props.onClick(); await nextTick();
    await library.setFilter("recent"); library.query = "影片";
    button("全部作品").props.onClick(); await nextTick();
    assert.equal(library.filter, "favorites"); assert.equal(library.query, "漫畫");
    assert.equal(library.statusFilter, "read");
    assert.equal(button("全部作品").props["aria-pressed"], true);
    button("影片").props.onClick(); await nextTick();
    assert.equal(library.filter, "recent"); assert.equal(library.query, "影片");
    assert.equal(button("影片").props["aria-pressed"], true);
    button("書庫").props.onClick(); await nextTick();
    assert.equal(library.filter, "all"); assert.equal(library.mediaFilter, "video");
    library.setMediaFilter("all"); library.filter = "favorites";
    library.section = "series"; library.activeSeries = "測試系列";
    button("全部作品").props.onClick(); await nextTick();
    assert.equal(library.section, "books"); assert.equal(library.activeSeries, null);
    assert.equal(library.filter, "favorites");
  } finally { app?.unmount(); await harness.cleanup(); }
});

test("new navigation layouts keep media, favorites, series and settings usable without resetting selection", async () => {
  const directory=await mkdtemp(new URL("../.navigation-test-",import.meta.url)); let app;
  try {
    await writeFile(`${directory}/stubs.mjs`, `
      import {reactive} from 'vue';
      export const library=reactive({books:[{id:1,format:'folder',favorite:true,available:true,lastReadAt:null},{id:2,format:'video',favorite:false,available:true,lastReadAt:123}],
        mediaFilter:'all',filter:'all',section:'books',activeSeries:null,selectedIds:[1],query:'保留搜尋',
        setFilter(value){this.filter=value},setMediaFilter(value){this.mediaFilter=value}});
      export const useLibraryStore=()=>library,groupSeries=()=>[];
    `);
    const Navigation=await compileComponent(directory,"LibraryNavigation",{"../stores/library":"./stubs.mjs","../lib/series":"./stubs.mjs"});
    const {library}=await import(pathToFileURL(`${directory}/stubs.mjs`));
    const props=reactive({mode:"workbench",busy:false});let settings=0;
    const root={children:[]};
    app=makeRenderer().createApp(Navigation,{...props,onSettings:()=>settings++});app.mount(root);await nextTick();
    const button=label=>find(root,node=>node.type==="button"&&textContent(node).startsWith(label));
    button("影片").props.onClick(); await nextTick(); assert.equal(library.mediaFilter,"video");
    assert.equal(button("系列").props.disabled,true);
    button("收藏").props.onClick(); await nextTick(); assert.equal(library.filter,"favorites");
    assert.equal(library.query,"保留搜尋");assert.deepEqual(library.selectedIds,[1]);
    button("設定").props.onClick(); assert.equal(settings,1);
    app.unmount();app=null;
    for (const mode of ["gallery","studio"]) {
      const nextRoot={children:[]}; app=makeRenderer().createApp(Navigation,{mode,busy:false});app.mount(nextRoot);await nextTick();
      assert.equal(find(nextRoot,node=>node.props?.class==="media-nav"),undefined);
      const libraryButton=find(nextRoot,node=>node.type==="button"&&textContent(node)==="書庫");
      libraryButton.props.onClick(); await nextTick(); assert.equal(library.filter,"all");
      assert.equal(library.query,"保留搜尋"); app.unmount();app=null;
    }
  } finally {app?.unmount();await rm(directory,{recursive:true,force:true});}
});

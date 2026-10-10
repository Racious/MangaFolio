import { test } from "node:test";
import assert from "node:assert/strict";
import { readFile, writeFile, mkdtemp, rm } from "node:fs/promises";
import { pathToFileURL } from "node:url";
import { nextTick } from "vue";
import { makeRenderer, find, textContent, compileComponent } from "./vue-harness.mjs";
import { createLibraryHarness } from "./library-harness.mjs";
const settle = async () => { for (let i=0;i<12;i++) await nextTick(); };

test("gallery modal shows real video opening failures and permits a successful retry", async t => {
  t.mock.timers.enable({ apis: ["setTimeout"] });
  const harness = await createLibraryHarness(); let app;
  const previousWindow = globalThis.window, previousDocument = globalThis.document;
  globalThis.window = { matchMedia: () => ({ matches: true, addEventListener() {}, removeEventListener() {} }), addEventListener() {}, removeEventListener() {} };
  globalThis.document = { activeElement: null, querySelector: () => null };
  try {
    const { directory, library, ipc } = harness;
    const movie = { id: 9, title: "測試影片", path: "D:/test.mp4", format: "video", available: true, tags: [], series: "", notes: "", readingStatus: "unread", lastReadAt: null };
    library.books = [movie]; ipc.options.list = async () => [movie];
    ipc.options.videoError = "無法開啟影片：播放器錯誤";
    const Panel = await compileComponent(directory, "BookDetailsPanel", Object.fromEntries([
      "../stores/library", "../stores/reader", "../api/media", "../lib/library", "../lib/series", "./BookCover.vue", "./BookmarksPanel.vue",
    ].map(path => [path, "./stubs.mjs"])));
    const root = { children: [] }; let closes = 0;
    app = makeRenderer().createApp(Panel, { book: movie, presentation: "modal", compact: false, onClose: () => closes++ });
    app.mount(root); await settle();
    const play = () => find(root, n => n.type === "button" && textContent(n) === "使用預設播放器開啟");
    await play().props.onClick(); await settle();
    assert.equal(find(root, n => n.type === "dialog").modal, true);
    const alert = find(root, n => n.props?.role === "alert");
    assert.ok(alert, "the failure must be visible inside the modal");
    assert.match(textContent(alert), /播放器錯誤/);
    assert.equal(closes, 0); assert.equal(play().props.disabled, false);
    ipc.options.videoError = null;
    await play().props.onClick(); await settle();
    assert.equal(closes, 1); assert.equal(library.error, "");
    assert.deepEqual(ipc.calls, [9, 9]);
  } finally { app?.unmount(); globalThis.window = previousWindow; globalThis.document = previousDocument; await harness.cleanup(); }
});

test("compact information drawer replaces covers directly, preserves cancel and reports failure", async () => {
  const directory=await mkdtemp(new URL("../.media-test-",import.meta.url));let app;
  const previousWindow=globalThis.window, previousDocument=globalThis.document;
  globalThis.window={matchMedia:()=>({matches:true,addEventListener(){},removeEventListener(){}}),addEventListener(){},removeEventListener(){}};
  globalThis.document={activeElement:null,querySelector:()=>null};
  try {
    await writeFile(`${directory}/stubs.mjs`, `
      import {reactive} from 'vue';
      export const options={path:null,fails:false},calls=[];
      export const library=reactive({books:[],revision:1,favoritePending:new Set(),managing:false,error:'',importNotice:'',
        refresh:async()=>{library.revision++},notifySuccess:message=>library.importNotice=message});
      export const useLibraryStore=()=>library, useReaderStore=()=>({flushProgress:async()=>calls.push('flush')});
      export {progressPercent,statusLabels,videoStatusLabels,displayPath} from '../src/lib/library.ts';
      export const nextVolume=()=>({reason:'沒有下一集'});
      export const pickCover=async()=>options.path,replaceCover=async(id,path)=>{if(options.fails)throw Error('無效圖片');calls.push([id,path])};
      export const showSourceLocation=async()=>{};
      export default {render:()=>null};
    `);
    const replacements=Object.fromEntries(['../stores/library','../stores/reader','../api/media','../lib/library','../lib/series','./BookCover.vue','./BookmarksPanel.vue'].map(p=>[p,'./stubs.mjs']));
    const Panel=await compileComponent(directory,'BookDetailsPanel',replacements);
    const {library,options,calls}=await import(pathToFileURL(`${directory}/stubs.mjs`));
    const root={children:[]}; app=makeRenderer().createApp(Panel,{compact:true,book:{id:9,title:'測試影片',format:'video',tags:[],available:false,readingStatus:'unread',notes:'',path:'D:/test.mp4'}});app.mount(root);await settle();
    const button=()=>find(root,n=>n.type==='button'&&textContent(n)==='更換封面');
    await button().props.onClick();assert.equal(calls.length,0);assert.equal(library.revision,1);
    options.path='D:/cover.png';await button().props.onClick();await settle();
    assert.deepEqual(calls,['flush',[9,'D:/cover.png']]);assert.equal(library.revision,2);assert.match(library.importNotice,/封面已更新/);
    options.fails=true;await button().props.onClick();await settle();assert.match(library.error,/無效圖片/);assert.equal(library.managing,false);
    assert.equal(library.revision,2);
  } finally {app?.unmount();globalThis.window=previousWindow;globalThis.document=previousDocument;await rm(directory,{recursive:true,force:true});}
});

test("cover editor saves immediately, refreshes the offline preview and resets automatic covers",async()=>{
  const directory=await mkdtemp(new URL("../.media-test-",import.meta.url));let app;
  try{
    await writeFile(`${directory}/stubs.mjs`,`
      import {reactive} from 'vue';
      export const calls=[], options={path:'D:/new.png'};
      export const library=reactive({revision:1,favoritePending:new Set(),managing:false,
        refresh:async()=>{library.revision++},notifySuccess:()=>{}});
      export const useLibraryStore=()=>library;
      export const useReaderStore=()=>({flushProgress:async()=>{}});
      export const displayPath=p=>p;
      export const pickCover=async()=>options.path;
      export const replaceCover=async(id,path)=>calls.push([id,path]);
      export const editBook=async()=>{};
      export const loadCover=async(id)=>'blob:cover-'+library.revision;
    `);
    await compileComponent(directory,"BookCover",{"../api/covers":"./stubs.mjs"});
    const Editor=await compileComponent(directory,"BookEditor",{
      "../api/library":"./stubs.mjs","../api/media":"./stubs.mjs","../lib/library":"./stubs.mjs",
      "../stores/library":"./stubs.mjs","../stores/reader":"./stubs.mjs","./BookCover.vue":"./BookCover.mjs",
    });
    const root={children:[]};app=makeRenderer().createApp(Editor,{book:{id:9,title:"離線影片",format:"video",available:false}});app.mount(root);await settle();
    const src=()=>find(root,n=>n.type==="img").props.src;assert.equal(src(),"blob:cover-1");
    await find(root,n=>n.type==="button"&&textContent(n)==="更換封面").props.onClick();await settle();
    assert.equal(src(),"blob:cover-2");
    await find(root,n=>n.type==="button"&&textContent(n)==="恢復自動封面").props.onClick();await settle();
    const stubs=await import(pathToFileURL(`${directory}/stubs.mjs`));assert.deepEqual(stubs.calls,[[9,"D:/new.png"],[9,null]]);
    stubs.options.path=null;await find(root,n=>n.type==="button"&&textContent(n)==="更換封面").props.onClick();await settle();
    assert.equal(stubs.calls.length,2);assert.equal(stubs.library.managing,false);
  }finally{app?.unmount();await rm(directory,{recursive:true,force:true});}
});

test("video card exposes external opening and hides comic page progress",async()=>{
  const directory=await mkdtemp(new URL("../.media-test-",import.meta.url));
  const previous=globalThis.IntersectionObserver;globalThis.IntersectionObserver=class{observe(){}disconnect(){}};
  let app;
  try{
    await writeFile(`${directory}/stubs.mjs`,`
      export const PhStar=()=>null,PhPencilSimple=()=>null,PhPlay=()=>null,PhBookOpen=()=>null;
      export const loadCover=async()=>null;
      export {progressPercent,statusLabels,videoStatusLabels,displayPath} from '../src/lib/library.ts';
    `);
    const Card=await compileComponent(directory,"BookCard",Object.fromEntries(["@phosphor-icons/vue","../api/covers","../lib/library"].map(p=>[p,"./stubs.mjs"])));
    const root={children:[]};let opened;
    app=makeRenderer().createApp(Card,{book:{id:8,title:"影片",format:"video",available:true,tags:[],readingStatus:"unread",lastReadAt:123,pageCount:1,lastIndex:0},opening:false,favoritePending:false,onOpen:id=>opened=id});
    app.mount(root);await settle();
    assert.match(textContent(root),/未看/);assert.match(textContent(root),/外部播放器/);assert.doesNotMatch(textContent(root),/100%|1 \/ 1 頁/);
    const cover=find(root,n=>n.type==="button"&&n.props?.class==="cover");assert.match(cover.props["aria-label"],/預設播放器/);
    cover.props.onClick({stopPropagation(){}});assert.equal(opened,8);
    assert.equal(find(root,n=>n.props?.class==="progress"),undefined);
  }finally{app?.unmount();globalThis.IntersectionObserver=previous;await rm(directory,{recursive:true,force:true});}
});

test("real import dialog previews recursive multiple roots, honors selections and remembers roots", async () => {
  const directory = await mkdtemp(new URL("../.media-test-",import.meta.url));
  let app;
  try {
    await writeFile(`${directory}/stubs.mjs`, `
      import { reactive } from 'vue';
      export const calls=[],options={rememberFails:false};
      export const library=reactive({managing:false,error:'',importNotice:'',importProgress:{completed:0,total:0},
        add:async paths=>calls.push(['add',paths]),cancelImport:()=>{}});
      export const useLibraryStore=()=>library;
      export const displayPath=p=>p;
      export const pickFolders=async()=>['D:/2026','E:/media'];
      export const pickMediaFiles=async()=>[];
      export const listRoots=async()=>[];
      export const forgetRoot=async p=>calls.push(['forget',p]);
      export const rememberRoots=async p=>{calls.push(['remember',p]);if(options.rememberFails)throw Error('主目錄已移除');};
      export const cancelScan=async()=>{};
      export const scanSources=async (paths,recursive)=>{calls.push(['scan',paths,recursive]);return {
        items:[{path:'D:/2026/book',title:'漫畫',format:'folder',selected:true,existing:false,warning:''},
          {path:'E:/media/movie.mp4',title:'影片',format:'video',selected:true,existing:false,warning:''},
          {path:'E:/media/old.mp4',title:'舊影片',format:'video',selected:false,existing:true,warning:''},
          {path:'E:/media/mixed',title:'混合',format:'folder',selected:false,existing:false,warning:'辨識不明'}],
        issues:[],canceled:false,truncated:false};};
    `);
    const Panel=await compileComponent(directory,"ImportPanel",Object.fromEntries(["../api/media","../lib/library","../stores/library"].map(p=>[p,"./stubs.mjs"])));
    const root={children:[]};let closed=0;
    app=makeRenderer().createApp(Panel,{mode:"roots",onClose:()=>closed++});app.mount(root);await settle();
    const stubs=await import(pathToFileURL(`${directory}/stubs.mjs`));
    assert.deepEqual(stubs.calls[0],["scan",["D:/2026","E:/media"],true]);
    assert.match(textContent(root),/已選 2 筆/);assert.match(textContent(root),/辨識不明/);
    const apply=find(root,n=>n.type==="button"&&textContent(n).startsWith("匯入所選"));
    await apply.props.onClick();await settle();
    assert.deepEqual(stubs.calls.slice(1),[["add",["D:/2026/book","E:/media/movie.mp4"]],["remember",["D:/2026","E:/media"]]]);
    assert.equal(closed,1);assert.equal(stubs.library.managing,false);
    stubs.options.rememberFails=true;
    await apply.props.onClick();await settle();
    assert.equal(closed,2,"import finishes even if root registration fails");
    assert.match(stubs.library.importNotice,/作品已匯入.*無法記住主目錄.*主目錄已移除/);
    assert.equal(stubs.library.error,'');
  } finally { app?.unmount();await rm(directory,{recursive:true,force:true}); }
});

test("real import dialog cancels scanning and permits only the partial preview afterwards",async()=>{
  const directory=await mkdtemp(new URL("../.media-test-",import.meta.url));let app;
  try{
    await writeFile(`${directory}/stubs.mjs`,`
      import { reactive } from 'vue';
      export const library=reactive({managing:false,error:'',importProgress:{},add:async()=>{}});
      export const useLibraryStore=()=>library;export const displayPath=p=>p;
      export const pickFolders=async()=>['D:/root'];export const pickMediaFiles=async()=>[];
      export const listRoots=async()=>[];export const rememberRoots=async()=>{};export const forgetRoot=async()=>{};
      let resolve;export const scanSources=()=>new Promise(r=>resolve=r);
      export const cancelScan=async()=>resolve({items:[],issues:[],canceled:true,truncated:false});
    `);
    const Panel=await compileComponent(directory,"ImportPanel",Object.fromEntries(["../api/media","../lib/library","../stores/library"].map(p=>[p,"./stubs.mjs"])));
    const root={children:[]};app=makeRenderer().createApp(Panel,{mode:"roots"});app.mount(root);await settle();
    const stubs=await import(pathToFileURL(`${directory}/stubs.mjs`));assert.equal(stubs.library.managing,true);
    await find(root,n=>n.type==="button"&&textContent(n)==="取消掃描").props.onClick();await settle();
    assert.equal(stubs.library.managing,false);assert.match(textContent(root),/掃描已取消/);
    assert.equal(find(root,n=>n.type==="button"&&textContent(n).startsWith("匯入所選")).props.disabled,true);
  }finally{app?.unmount();await rm(directory,{recursive:true,force:true});}
});

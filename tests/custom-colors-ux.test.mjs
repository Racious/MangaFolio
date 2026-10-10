import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, writeFile, rm } from "node:fs/promises";
import { pathToFileURL } from "node:url";
import { nextTick } from "vue";
import { makeRenderer, compileComponent, find, textContent } from "./vue-harness.mjs";
test("color text entry saves valid values immediately, rejects invalid values and survives layout changes", async () => {
  const directory=await mkdtemp(new URL("../.color-test-",import.meta.url)); let app;
  try {
    await writeFile(`${directory}/stubs.mjs`, `
      import {reactive} from 'vue';
      import {freshAppearance} from '../src/lib/appearance.ts';
      export {styles,validColor} from '../src/lib/appearance.ts';
      export const appearance=reactive({settings:{...freshAppearance(),palette:'custom'},set(key,value){this.settings[key]=value}});
      export const useAppearanceStore=()=>appearance;
    `);
    const Component=await compileComponent(directory,"AppearanceSettings",{"../stores/appearance":"./stubs.mjs","../lib/appearance":"./stubs.mjs"});
    const {appearance}=await import(pathToFileURL(`${directory}/stubs.mjs`));const root={children:[]};
    app=makeRenderer().createApp(Component);app.mount(root);await nextTick();
    const input=find(root,node=>node.props?.["aria-label"] === "背景色碼");
    input.props.onInput({type:"input",target:{value:"#ffffff"}});await nextTick();assert.equal(appearance.settings.colors.background,"#ffffff");
    const invalid={value:"#123"};input.props.onChange({type:"change",target:invalid});await nextTick();
    assert.equal(invalid.value,"#ffffff");assert.match(textContent(root),/六位色碼/);
    find(root,node=>node.props?.["aria-label"] === "介面風格").props.onChange({target:{value:"gallery"}});await nextTick();
    assert.equal(appearance.settings.style,"gallery");assert.equal(appearance.settings.colors.background,"#ffffff");
  } finally {app?.unmount();await rm(directory,{recursive:true,force:true});}
});

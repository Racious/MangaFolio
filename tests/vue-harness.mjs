import { readFile, writeFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { parse, compileScript } from '@vue/compiler-sfc';
import ts from 'typescript';
import { createRenderer } from 'vue';
export function makeRenderer() {
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
export const find = (node, match) => match(node) ? node : node.children?.map(child => find(child, match)).find(Boolean);
export const textContent = node => (node.text ?? "") + (node.children?.map(textContent).join("") ?? "");
export async function compileComponent(directory, name, replacements) {
  const source = await readFile(new URL(`../src/components/${name}.vue`, import.meta.url), "utf8");
  const { descriptor } = parse(source);
  const compiled = compileScript(descriptor, { id: name, inlineTemplate: true, templateOptions: { compilerOptions: { hoistStatic: false } } });
  let js = ts.transpileModule(compiled.content, {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext },
  }).outputText;
  for (const [from, to] of Object.entries(replacements)) js = js.replaceAll(`"${from}"`, `"${to}"`);
  await writeFile(`${directory}/${name}.mjs`, js);
  return (await import(pathToFileURL(`${directory}/${name}.mjs`))).default;
}

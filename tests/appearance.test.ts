import { test } from "node:test";
import assert from "node:assert/strict";
import {
  defaultAppearance,
  loadAppearance,
  parseAppearance,
  saveAppearance,
} from "../src/lib/appearance.ts";
test("appearance restores known settings and tolerates missing or corrupt storage", () => {
  for (const raw of [null, "{bad", "null", "[]", "42"])
    assert.deepEqual(parseAppearance(raw), defaultAppearance);
  assert.deepEqual(
    parseAppearance(
      '{"theme":"dark","view":"compact","density":"compact","coverSize":"large"}',
    ),
    {
      style: "calm",
      theme: "dark",
      view: "compact",
      density: "compact",
      coverSize: "large",
    },
  );
  assert.equal(
    parseAppearance('{"theme":"execute-css","view":"detail"}').theme,
    "system",
  );
  assert.equal(
    parseAppearance('{"theme":"execute-css","view":"detail"}').view,
    "detail",
  );
  assert.deepEqual(
    loadAppearance({
      getItem() {
        throw Error("blocked");
      },
    }),
    defaultAppearance,
  );
  assert.equal(
    saveAppearance(
      {
        setItem() {
          throw Error("full");
        },
      },
      defaultAppearance,
    ),
    false,
  );
  let raw = "";
  assert.equal(
    saveAppearance(
      {
        setItem(_key, value) {
          raw = value;
        },
      },
      { ...defaultAppearance, view: "detail" },
    ),
    true,
  );
  assert.equal(loadAppearance({ getItem: () => raw }).view, "detail");
});

test("styles are independent from view and theme and old preferences gain default style", () => {
  for (const style of ["calm", "catalog", "night"]) {
    for (const view of ["grid", "detail", "compact"]) {
      const value = parseAppearance(
        JSON.stringify({ style, view, theme: "light" }),
      );
      assert.equal(value.style, style);
      assert.equal(value.view, view);
      assert.equal(value.theme, "light");
    }
  }
  assert.equal(
    parseAppearance('{"style":"unknown","view":"detail"}').style,
    "calm",
  );
});

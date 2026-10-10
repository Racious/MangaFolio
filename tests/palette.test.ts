import { test } from "node:test";
import assert from "node:assert/strict";
import { parseAppearance, freshAppearance } from "../src/lib/appearance.ts";
import { paletteTokens, contrast, resolvePalette } from "../src/lib/palette.ts";

test("all three new layouts persist independently from customized colors and old settings", () => {
  for (const style of ["workbench", "gallery", "studio"]) {
    const saved = parseAppearance(JSON.stringify({ style, palette: "custom", colors: { background: "#ffccaa", panel: "#101010", accent: "#999999" } }));
    assert.equal(saved.style, style);
    assert.equal(saved.colors.panel, "#101010");
    assert.deepEqual(resolvePalette(saved, false), resolvePalette(saved, true));
  }
  const old = parseAppearance('{"style":"catalog","theme":"dark"}');
  assert.equal(old.style, "catalog");
  assert.equal(resolvePalette(old, true), null);
  const invalid = parseAppearance('{"colors":{"accent":"url(javascript:bad)","panel":"#123"}}');
  assert.deepEqual(invalid.colors, freshAppearance().colors);
  invalid.colors.accent = "#ffffff";
  assert.notEqual(freshAppearance().colors.accent, "#ffffff");
});
test("foreground contrast follows each surface and accent, even with opposite custom colors", () => {
  for (const background of ["#ffffff", "#000000", "#777777", "#ffccaa", "#99cc00"]) {
    const tokens = paletteTokens({ background, panel: "#000000", accent: "#ffcc00" });
    assert.ok(contrast(tokens["--text"], background) >= 4.5);
    assert.ok(contrast(tokens["--panel-text"], "#000000") >= 4.5);
    assert.ok(contrast(tokens["--accent-ink"], "#ffcc00") >= 4.5);
  }
});

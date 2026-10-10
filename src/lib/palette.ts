import type { Appearance } from "./appearance";

type Colors = Appearance["colors"];
const rgb = (hex: string) => [1, 3, 5].map(index => parseInt(hex.slice(index, index + 2), 16));
export function luminance(hex: string): number {
  return rgb(hex).map(value => {
    const channel = value / 255;
    return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
  }).reduce((sum, value, index) => sum + value * [0.2126, 0.7152, 0.0722][index], 0);
}
export function contrast(a: string, b: string): number {
  const values = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (values[0] + 0.05) / (values[1] + 0.05);
}
const ink = (background: string) => {
  const dark = contrast("#17191d", background) >= 4.5 ? "#17191d" : "#000000";
  return contrast("#ffffff", background) > contrast(dark, background) ? "#ffffff" : dark;
};
function mix(a: string, b: string, weight: number): string {
  const other = rgb(b);
  return "#" + rgb(a).map((value, index) => Math.round(value * weight + other[index] * (1 - weight)).toString(16).padStart(2, "0")).join("");
}
export function paletteTokens(colors: Colors): Record<string, string> {
  const text = ink(colors.background), panelText = ink(colors.panel);
  const readableAccent = contrast(colors.accent, colors.background) >= 4.5 ? colors.accent : text;
  return {
    "--bg": colors.background, "--panel": colors.panel, "--accent": colors.accent,
    "--text": text, "--text-dim": mix(text, colors.background, 0.8),
    "--background-text": text,
    "--panel-text": panelText, "--panel-muted": mix(panelText, colors.panel, 0.8),
    "--panel-soft": mix(panelText, colors.panel, 0.06), "--panel-border": mix(panelText, colors.panel, 0.55),
    "--panel-accent": contrast(colors.accent, colors.panel) >= 4.5 ? colors.accent : panelText,
    "--bg-soft": mix(text, colors.background, 0.06),
    "--border": mix(text, colors.background, 0.55),
    "--accent-soft": readableAccent, "--accent-ink": ink(colors.accent),
    "--focus": text, "--cover-bg": colors.panel,
    "--red": luminance(colors.background) < 0.3 ? "#ffb9b9" : "#a52337",
  };
}
export function resolvePalette(settings: Appearance, dark: boolean): Colors | null {
  if (settings.palette === "custom") return settings.colors;
  const palette = settings.palette === "recommended"
    ? ({ workbench: "jade", gallery: "plum", studio: "paper" } as Record<string, string>)[settings.style]
    : settings.palette;
  if (!palette) return null; // Existing styles retain their original palettes.
  const presets: Record<string, Colors[]> = {
    jade: [{ background: "#f2f5f3", panel: "#ffffff", accent: "#316b5c" }, { background: "#20252d", panel: "#292f39", accent: "#9dcdbf" }],
    plum: [{ background: "#f8f4ef", panel: "#ffffff", accent: "#805d34" }, { background: "#262329", panel: "#312d34", accent: "#d8c4a5" }],
    paper: [{ background: "#f6f4ef", panel: "#fffefa", accent: "#346f65" }, { background: "#202924", panel: "#29352e", accent: "#a5cdb9" }],
  };
  return presets[palette]?.[dark ? 1 : 0] ?? null;
}

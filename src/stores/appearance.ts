import { defineStore } from "pinia";
import {
  freshAppearance,
  loadAppearance,
  saveAppearance,
  type Appearance,
} from "../lib/appearance";
import { paletteTokens, resolvePalette } from "../lib/palette";
const colorProperties = Object.keys(paletteTokens(freshAppearance().colors));
export const useAppearanceStore = defineStore("appearance", {
  state: () => ({ settings: freshAppearance(), saveFailed: false }),
  actions: {
    load() {
      try {
        this.settings = loadAppearance(localStorage);
      } catch {
        this.settings = freshAppearance();
      }
    },
    set<K extends keyof Appearance>(key: K, value: Appearance[K]) {
      this.settings = { ...this.settings, [key]: value };
      try {
        this.saveFailed = !saveAppearance(localStorage, this.settings);
      } catch {
        this.saveFailed = true;
      }
    },
    apply(systemDark: boolean) {
      const root = document.documentElement;
      root.dataset.style = this.settings.style;
      root.dataset.theme =
        this.settings.theme === "system"
          ? systemDark
            ? "dark"
            : "light"
          : this.settings.theme;
      root.dataset.density = this.settings.density;
      root.dataset.coverSize = this.settings.coverSize;
      root.dataset.palette = this.settings.palette;
      const colors = resolvePalette(this.settings, root.dataset.theme === "dark");
      for (const property of colorProperties) root.style.removeProperty(property);
      if (colors) {
        for (const [property, value] of Object.entries(paletteTokens(colors))) root.style.setProperty(property, value);
      }
      root.dataset.customColors = colors ? "true" : "false";
    },
  },
});

import { defineStore } from "pinia";
import {
  defaultAppearance,
  loadAppearance,
  saveAppearance,
  type Appearance,
} from "../lib/appearance";
export const useAppearanceStore = defineStore("appearance", {
  state: () => ({ settings: { ...defaultAppearance }, saveFailed: false }),
  actions: {
    load() {
      try {
        this.settings = loadAppearance(localStorage);
      } catch {
        this.settings = { ...defaultAppearance };
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
    },
  },
});

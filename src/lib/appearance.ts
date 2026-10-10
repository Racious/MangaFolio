export const styles = [
  { value: "workbench", label: "收藏工作台", description: "完整側欄與作品網格，方便整理大量收藏" },
  { value: "gallery", label: "封面藝廊", description: "頂部導覽與寬幅封面牆，專注瀏覽作品" },
  { value: "studio", label: "資訊書架", description: "精簡導覽與作品資訊側欄，瀏覽與整理並行" },
  { value: "calm", label: "靜謐書架", description: "暖色紙感，適合日常閱讀" },
  {
    value: "catalog",
    label: "目錄工作台",
    description: "清晰線條，適合整理大型書庫",
  },
  { value: "night", label: "夜讀書房", description: "金色重點，適合專注閱讀" },
] as const;
export interface Appearance {
  style: (typeof styles)[number]["value"];
  theme: "light" | "dark" | "system";
  view: "grid" | "detail" | "compact";
  density: "comfortable" | "compact";
  coverSize: "small" | "medium" | "large";
  readerPinned: boolean;
  palette: "recommended" | "jade" | "plum" | "paper" | "custom";
  colors: { background: string; panel: string; accent: string };
}
export const appearanceKey = "mangafolio.appearance.v1";
export const defaultAppearance: Appearance = {
  style: "night",
  theme: "system",
  view: "grid",
  density: "comfortable",
  coverSize: "medium",
  readerPinned: false,
  palette: "recommended",
  colors: { background: "#20252d", panel: "#292f39", accent: "#9dcdbf" },
};
export const freshAppearance = (): Appearance => ({ ...defaultAppearance, colors: { ...defaultAppearance.colors } });
export const validColor = (value: unknown): value is string => typeof value === "string" && /^#[\da-f]{6}$/i.test(value);
export function parseAppearance(raw: string | null): Appearance {
  try {
    const data = JSON.parse(raw ?? "null");
    if (!data || typeof data !== "object" || Array.isArray(data))
      return freshAppearance();
    const allowed = {
      style: styles.map((s) => s.value) as readonly string[],
      theme: ["light", "dark", "system"],
      view: ["grid", "detail", "compact"],
      density: ["comfortable", "compact"],
      coverSize: ["small", "medium", "large"],
      palette: ["recommended", "jade", "plum", "paper", "custom"],
    };
    const result = freshAppearance();
    for (const key of Object.keys(allowed) as (keyof typeof allowed)[]) {
      if (allowed[key].includes(data[key]))
        Object.assign(result, { [key]: data[key] });
    }
    if (typeof data.readerPinned === "boolean")
      result.readerPinned = data.readerPinned;
    for (const key of ["background", "panel", "accent"] as const) {
      if (validColor(data.colors?.[key])) result.colors[key] = data.colors[key];
    }
    return result;
  } catch {
    return freshAppearance();
  }
}
export function loadAppearance(storage: Pick<Storage, "getItem">): Appearance {
  try {
    return parseAppearance(storage.getItem(appearanceKey));
  } catch {
    return freshAppearance();
  }
}
export function saveAppearance(
  storage: Pick<Storage, "setItem">,
  settings: Appearance,
): boolean {
  try {
    storage.setItem(appearanceKey, JSON.stringify(settings));
    return true;
  } catch {
    return false;
  }
}

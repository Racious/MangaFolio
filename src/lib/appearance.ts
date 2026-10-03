export const styles = [
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
}
export const appearanceKey = "mangafolio.appearance.v1";
export const defaultAppearance: Appearance = {
  style: "calm",
  theme: "system",
  view: "grid",
  density: "comfortable",
  coverSize: "medium",
};
export function parseAppearance(raw: string | null): Appearance {
  try {
    const data = JSON.parse(raw ?? "null");
    if (!data || typeof data !== "object" || Array.isArray(data))
      return { ...defaultAppearance };
    const allowed = {
      style: styles.map((s) => s.value) as readonly string[],
      theme: ["light", "dark", "system"],
      view: ["grid", "detail", "compact"],
      density: ["comfortable", "compact"],
      coverSize: ["small", "medium", "large"],
    };
    const result = { ...defaultAppearance };
    for (const key of Object.keys(allowed) as (keyof Appearance)[]) {
      if (allowed[key].includes(data[key]))
        Object.assign(result, { [key]: data[key] });
    }
    return result;
  } catch {
    return { ...defaultAppearance };
  }
}
export function loadAppearance(storage: Pick<Storage, "getItem">): Appearance {
  try {
    return parseAppearance(storage.getItem(appearanceKey));
  } catch {
    return { ...defaultAppearance };
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

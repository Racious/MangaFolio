// 封裝所有 Tauri IPC 呼叫，讓前端其餘部分不直接接觸 invoke。

import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";

export interface BookInfo {
  title: string;
  pageCount: number;
  pages: string[];
  startIndex: number;
}

/** 縮放配合模式。 */
export type FitMode = "window" | "width" | "height" | "original" | "fixed";

export interface RenderRequest {
  index: number;
  mode: FitMode;
  viewportW: number;
  viewportH: number;
  fixedScale: number;
}

/** 開啟指定路徑（資料夾／壓縮檔／單張圖片）。 */
export function openPath(path: string): Promise<BookInfo> {
  return invoke<BookInfo>("open_path", { path });
}

/**
 * 請後端依縮放模式以 Lanczos3 處理指定頁面，回傳可供 <img> 使用的物件 URL。
 * 後端統一輸出 PNG。
 */
export async function renderPageUrl(req: RenderRequest): Promise<string> {
  const bytes = await invoke<ArrayBuffer>("render_page", { req });
  const blob = new Blob([bytes], { type: "image/png" });
  const url = URL.createObjectURL(blob);
  // 先完成解碼再回傳：確保換頁時影像已就緒，不出現空白閃爍。
  try {
    const img = new Image();
    img.src = url;
    await img.decode();
  } catch {
    /* 解碼失敗則交由 <img> 元素自行載入 */
  }
  return url;
}

/** 彈出系統對話框選擇資料夾。 */
export function pickFolder(): Promise<string | null> {
  return openDialog({ directory: true, multiple: false }) as Promise<string | null>;
}

/** 彈出系統對話框選擇壓縮檔或圖片。 */
export function pickFile(): Promise<string | null> {
  return openDialog({
    multiple: false,
    filters: [
      { name: "漫畫／圖集", extensions: ["zip", "cbz", "jpg", "jpeg", "jfif", "png", "gif", "webp", "bmp"] },
      { name: "所有檔案", extensions: ["*"] },
    ],
  }) as Promise<string | null>;
}

import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
export const videoExtensions = ["mp4", "mkv", "avi", "mov", "wmv", "webm", "m4v", "mpg", "mpeg", "ts", "m2ts"];
export interface ImportCandidate {
  path: string; title: string; format: "folder" | "cbz" | "video";
  selected: boolean; existing: boolean; warning: string;
}
export interface ScanResult { items: ImportCandidate[]; issues: string[]; canceled: boolean; truncated: boolean }
export const scanSources = (paths: string[], recursive: boolean) => invoke<ScanResult>("scan_library_sources", { paths, recursive });
export const cancelScan = () => invoke<void>("cancel_library_scan");
export const listRoots = () => invoke<string[]>("list_library_roots");
export const rememberRoots = (paths: string[]) => invoke<void>("remember_library_roots", { paths });
export const forgetRoot = (path: string) => invoke<void>("forget_library_root", { path });
export async function pickFolders(): Promise<string[] | null> {
  const paths = await open({ directory: true, multiple: true });
  return paths === null ? null : Array.isArray(paths) ? paths : [paths];
}
export async function pickMediaFiles(): Promise<string[] | null> {
  const paths = await open({ multiple: true, filters: [
    { name: "漫畫與影片", extensions: ["zip", "cbz", ...videoExtensions] },
    { name: "漫畫壓縮檔", extensions: ["zip", "cbz"] },
    { name: "影片", extensions: videoExtensions },
  ] });
  return paths === null ? null : Array.isArray(paths) ? paths : [paths];
}
export const openVideo = (id: number) => invoke<void>("open_library_video", { id });
export const showSourceLocation = (id: number) => invoke<void>("show_library_source_location", { id });
export const replaceCover = (id: number, path: string | null) => invoke<void>("replace_library_cover", { id, path });
export async function pickCover(): Promise<string | null> {
  const path = await open({ multiple: false, filters: [{ name: "封面圖片", extensions: ["jpg","jpeg","jfif","png","gif","webp","bmp"] }] });
  return typeof path === "string" ? path : null;
}

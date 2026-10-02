import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { FitMode } from "./backend";

export interface ReaderPreferences {
  direction: "rtl" | "ltr";
  pageMode: "single" | "double";
  zoom: FitMode;
  fixedScale: number;
  doubleCover: boolean;
  transition: "book" | "none" | "slide" | "fade";
}

export interface LibraryBook {
  id: number;
  path: string;
  title: string;
  format: "folder" | "cbz";
  pageCount: number;
  favorite: boolean;
  lastIndex: number;
  lastPageName: string | null;
  lastReadAt: number | null;
  preferences: ReaderPreferences;
  available: boolean;
}

export interface ReadingProgress {
  id: number;
  index: number;
  pageName: string;
  preferences: ReaderPreferences;
}

export const listLibrary = () => invoke<LibraryBook[]>("list_library");
export const importBook = (path: string) =>
  invoke<LibraryBook>("import_book", { path });
export const setFavorite = (id: number, favorite: boolean) =>
  invoke<void>("set_favorite", { id, favorite });
export const saveProgress = (progress: ReadingProgress) =>
  invoke<void>("save_reading_progress", { ...progress });
export const coverBytes = (id: number) =>
  invoke<ArrayBuffer>("library_cover", { id });
export async function pickBookFiles(): Promise<string[] | null> {
  return open({
    multiple: true,
    filters: [{ name: "漫畫壓縮檔", extensions: ["zip", "cbz"] }],
  }) as Promise<string[] | null>;
}

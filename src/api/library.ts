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

export type ReadingStatus = "unread" | "reading" | "read";
export interface Tag {
  id: number;
  name: string;
}
export interface BookDetails {
  customTitle: string;
  series: string;
  volume: string;
  notes: string;
}
export interface LibraryBook extends BookDetails {
  id: number;
  path: string;
  title: string;
  sourceTitle: string;
  readingStatus: ReadingStatus;
  statusManual: boolean;
  tags: Tag[];
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
export const removeBooks = (ids: number[]) =>
  invoke<void>("remove_library_books", { ids });
export const favoriteBooks = (ids: number[], favorite: boolean) =>
  invoke<void>("favorite_library_books", { ids, favorite });
export const relinkBook = (id: number, path: string) =>
  invoke<LibraryBook>("relink_library_book", { id, path });
export const exportBackup = (path: string) =>
  invoke<void>("export_library_backup", { path });
export const restoreBackup = (path: string) =>
  invoke<{ added: number; skipped: number }>("restore_library_backup", {
    path,
  });
export async function pickBookFiles(): Promise<string[] | null> {
  return open({
    multiple: true,
    filters: [{ name: "漫畫壓縮檔", extensions: ["zip", "cbz"] }],
  }) as Promise<string[] | null>;
}

export const importBookResult = (path: string) =>
  invoke<{ book: LibraryBook; kind: "added" | "updated" }>(
    "import_book_result",
    { path },
  );
export const editBook = (id: number, details: BookDetails) =>
  invoke<LibraryBook>("edit_library_book", { id, details });
export const setReadingStatus = (
  ids: number[],
  status: ReadingStatus | "auto",
) => invoke<void>("set_reading_status", { ids, status });
export const listTags = () => invoke<Tag[]>("list_tags");
export const createTag = (name: string) => invoke<Tag>("create_tag", { name });
export const renameTag = (id: number, name: string) =>
  invoke<void>("rename_tag", { id, name });
export const deleteTag = (id: number) => invoke<void>("delete_tag", { id });
export const assignTag = (ids: number[], tagId: number, add: boolean) =>
  invoke<void>("assign_book_tag", { ids, tagId, add });

export interface Bookmark {
  id: number;
  bookId: number;
  pageName: string;
  pageIndex: number;
  name: string;
  note: string;
}
export interface BackupSettings {
  enabled: boolean;
  retention: number;
  lastSuccess: number | null;
  lastError: string;
  customDirectory: string | null;
  directory: string;
  directoryWarning: string;
}
export interface RestorePreview {
  added: number;
  skipped: number;
  conflicts: number;
  unsupported: number;
  issues: string[];
  canRestore: boolean;
  version: number | null;
}
export const assignSeries = (ids: number[], name: string) =>
  invoke<void>("assign_book_series", { ids, name });
export const listBookmarks = (bookId: number) =>
  invoke<Bookmark[]>("list_bookmarks", { bookId });
export const saveBookmark = (bookmark: Bookmark) =>
  invoke<Bookmark>("save_bookmark", { bookmark });
export const deleteBookmark = (id: number, bookId: number) =>
  invoke<void>("delete_bookmark", { id, bookId });
export const previewBackup = (path: string) =>
  invoke<RestorePreview>("preview_library_backup", { path });
export const getBackupSettings = () =>
  invoke<BackupSettings>("get_backup_settings");
export const getBackupDirectory = () => invoke<string>("get_backup_directory");
export const openBackupDirectory = () => invoke<void>("open_backup_directory");
export const setBackupDirectory = (path: string | null) =>
  invoke<BackupSettings>("set_backup_directory", { path });
export const setBackupSettings = (enabled: boolean, retention: number) =>
  invoke<BackupSettings>("set_backup_settings", { enabled, retention });
export const runAutomaticBackup = (force = false) =>
  invoke<BackupSettings>("run_automatic_backup", { force });

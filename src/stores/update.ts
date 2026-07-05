// 自動更新狀態：檢查新版、下載安裝、重啟套用。
//
// 註：本 store 採 setup 風格（有別於 reader.ts 的 options 風格）。因為 `check()` 回傳的
// Update 物件內含方法（downloadAndInstall 等），須以 shallowRef 持有——放進 options state
// 會被 Pinia 深度響應包裹而破壞其原型方法。Pinia 官方允許兩種風格並存。

import { defineStore } from "pinia";
import { ref, shallowRef } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

/** 發行頁（手動更新的退路：自動更新失敗時導向此處）。 */
const RELEASES_URL = "https://github.com/Racious/MangaFolio/releases/latest";

interface CheckOptions {
  /** 靜默：檢查失敗不顯示錯誤（用於開機自動檢查）。 */
  silent?: boolean;
  /** 發現新版即自動下載安裝（本專案預設 false，交由使用者確認）。 */
  autoInstall?: boolean;
}

export const useUpdateStore = defineStore("update", () => {
  const currentVersion = ref("");
  const latestVersion = ref<string | null>(null);
  const releaseNotes = ref<string | null>(null);
  const releaseDate = ref<string | null>(null);
  const checking = ref(false);
  const installing = ref(false);
  const updateAvailable = ref(false);
  const statusMessage = ref<string | null>(null);
  const errorMessage = ref<string | null>(null);
  /** 0–100 的下載百分比；null 表示尚無進度（總長度未知或未開始）。 */
  const downloadProgress = ref<number | null>(null);
  /** 待安裝的更新物件（含方法，故用 shallowRef）。 */
  const pendingUpdate = shallowRef<Update | null>(null);

  async function loadCurrentVersion() {
    currentVersion.value = await getVersion();
  }

  /** 檢查更新。silent 用於開機自動檢查（失敗不打擾）。 */
  async function checkForUpdates(options: CheckOptions = {}) {
    if (checking.value || installing.value) return;

    checking.value = true;
    errorMessage.value = null;
    statusMessage.value = options.silent ? null : "檢查更新中…";

    try {
      if (!currentVersion.value) await loadCurrentVersion();

      const update = await check();
      pendingUpdate.value = update;
      updateAvailable.value = !!update;
      latestVersion.value = update?.version ?? null;
      releaseNotes.value = update?.body ?? null;
      releaseDate.value = update?.date ?? null;

      if (!update) {
        statusMessage.value = options.silent ? null : "已是最新版本。";
        return;
      }

      statusMessage.value = `發現新版本 ${update.version}`;
      if (options.autoInstall) await installUpdate();
    } catch (error) {
      // 靜默檢查（開機）失敗不打擾使用者：僅手動檢查才顯示錯誤彈窗。
      if (!options.silent) {
        errorMessage.value = error instanceof Error ? error.message : String(error);
      }
      statusMessage.value = null;
    } finally {
      checking.value = false;
    }
  }

  /** 下載並安裝待處理的更新，完成後重啟。 */
  async function installUpdate() {
    if (!pendingUpdate.value || installing.value) return;

    installing.value = true;
    downloadProgress.value = null;
    errorMessage.value = null;
    statusMessage.value = "下載更新中…";

    try {
      let contentLength = 0;
      let downloaded = 0;

      await pendingUpdate.value.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            contentLength = event.data.contentLength ?? 0;
            downloadProgress.value = 0;
            break;
          case "Progress":
            downloaded += event.data.chunkLength;
            downloadProgress.value =
              contentLength > 0 ? Math.round((downloaded / contentLength) * 100) : null;
            break;
          case "Finished":
            downloadProgress.value = 100;
            break;
        }
      });

      statusMessage.value = "更新完成，即將重新啟動…";
      await relaunch();
    } catch (error) {
      errorMessage.value = error instanceof Error ? error.message : String(error);
      downloadProgress.value = null;
      installing.value = false;
    }
    // 成功時會 relaunch，不重置 installing（進程即將結束）。
  }

  /** 使用者關閉更新提示（不安裝）。 */
  function dismiss() {
    updateAvailable.value = false;
    statusMessage.value = null;
    errorMessage.value = null;
  }

  /** 自動更新失敗時，導向 GitHub 發行頁供手動下載。 */
  async function openReleasePage() {
    await openUrl(RELEASES_URL);
  }

  return {
    currentVersion,
    latestVersion,
    releaseNotes,
    releaseDate,
    checking,
    installing,
    updateAvailable,
    statusMessage,
    errorMessage,
    downloadProgress,
    loadCurrentVersion,
    checkForUpdates,
    installUpdate,
    dismiss,
    openReleasePage,
  };
});

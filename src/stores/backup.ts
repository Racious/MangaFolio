import { defineStore } from "pinia";
import {
  getBackupSettings,
  runAutomaticBackup,
  setBackupSettings,
  setBackupDirectory,
  type BackupSettings,
} from "../api/library";
export const useBackupStore = defineStore("backup", {
  state: () => ({
    settings: {
      enabled: false,
      retention: 5,
      lastSuccess: null,
      lastError: "",
      customDirectory: null,
      directory: "",
      directoryWarning: "",
    } as BackupSettings,
    pending: false,
    error: "",
  }),
  actions: {
    async refresh() {
      try {
        const settings = await getBackupSettings();
        if (settings) this.settings = settings;
      } catch (e) {
        this.error = String(e);
      }
    },
    async configure(enabled: boolean, retention: number) {
      if (this.pending) return;
      this.pending = true;
      this.error = "";
      try {
        this.settings = await setBackupSettings(enabled, retention);
      } catch (e) {
        this.error = String(e);
      } finally {
        this.pending = false;
      }
    },
    async changeDirectory(path: string | null) {
      if (this.pending) return false;
      this.pending = true;
      this.error = "";
      try {
        this.settings = await setBackupDirectory(path);
        return true;
      } catch (e) {
        this.error = String(e);
        return false;
      } finally {
        this.pending = false;
      }
    },
    async maintain(force = false) {
      if (this.pending) return;
      this.pending = true;
      this.error = "";
      try {
        const settings = await runAutomaticBackup(force);
        if (settings) this.settings = settings;
      } catch (e) {
        this.error = String(e);
        await this.refresh();
      } finally {
        this.pending = false;
      }
    },
  },
});

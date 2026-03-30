import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// 注意：时间格式化已由后端统一处理，前端不再需要格式化函数

export interface QueryParams {
  dbPath: string;
  startTime: number;
  endTime: number;
}

export interface SyncDatabaseParams {
  dbPath: string;
  targetPath?: string;
  startTime?: number;
  endTime?: number;
}

export type SourceMode = "remote_sync" | "local_import";

interface QuerySourceConfig {
  sourceMode: SourceMode;
  remoteDbPath: string;
  syncTargetPath: string;
  importedDbPath: string;
  activeDbPath: string;
  lastReadyAt: string | null;
}

export interface ExportWideTableParams {
  dbPath: string;
  startTime: number;
  endTime: number;
  outputPath: string;
}

export interface ExportDemandResultsParams {
  dbPath: string;
  startTime: number;
  endTime: number;
  outputPath: string;
}

export interface QueryResult {
  columns: string[];
  rows: Record<string, any>[];
  totalRows: number;
}

const SOURCE_CONFIG_KEY = "query-source-config";

const defaultRemoteDbPath = "/mnt/analysis/data/device_data.db";

const defaultSourceConfig: QuerySourceConfig = {
  sourceMode: "remote_sync",
  remoteDbPath: defaultRemoteDbPath,
  syncTargetPath: "",
  importedDbPath: "",
  activeDbPath: "",
  lastReadyAt: null,
};

function loadSourceConfigFromStorage(): QuerySourceConfig {
  try {
    const saved = localStorage.getItem(SOURCE_CONFIG_KEY);
    if (!saved) {
      return { ...defaultSourceConfig };
    }

    const parsed = JSON.parse(saved) as Partial<QuerySourceConfig>;
    return {
      ...defaultSourceConfig,
      ...parsed,
    };
  } catch {
    return { ...defaultSourceConfig };
  }
}

export const useQueryStore = defineStore("query", {
  state: () => ({
    results: null as QueryResult | null,
    loading: false,
    error: null as string | null,
    progress: 0,
    progressMessage: "",
    logs: [] as string[],
    exportedRows: 0,
    exportedPath: null as string | null,
    dbSynced: false,
    dbSyncTime: null as string | null,
    syncing: false,
    syncProgress: 0,
    syncProgressMessage: "",
    sourceMode: defaultSourceConfig.sourceMode as SourceMode,
    remoteDbPath: defaultSourceConfig.remoteDbPath,
    syncTargetPath: defaultSourceConfig.syncTargetPath,
    importedDbPath: defaultSourceConfig.importedDbPath,
    activeDbPath: defaultSourceConfig.activeDbPath,
    lastReadyAt: defaultSourceConfig.lastReadyAt,
  }),

  actions: {
    loadSourceConfig() {
      const config = loadSourceConfigFromStorage();
      this.sourceMode = config.sourceMode;
      this.remoteDbPath = config.remoteDbPath;
      this.syncTargetPath = config.syncTargetPath;
      this.importedDbPath = config.importedDbPath;
      this.activeDbPath = config.activeDbPath;
      this.lastReadyAt = config.lastReadyAt;
    },

    saveSourceConfig() {
      const config: QuerySourceConfig = {
        sourceMode: this.sourceMode,
        remoteDbPath: this.remoteDbPath,
        syncTargetPath: this.syncTargetPath,
        importedDbPath: this.importedDbPath,
        activeDbPath: this.activeDbPath,
        lastReadyAt: this.lastReadyAt,
      };
      localStorage.setItem(SOURCE_CONFIG_KEY, JSON.stringify(config));
    },

    setSourceMode(mode: SourceMode) {
      this.sourceMode = mode;
      this.saveSourceConfig();
    },

    setRemoteDbPath(path: string) {
      this.remoteDbPath = path;
      this.saveSourceConfig();
    },

    setSyncTargetPath(path: string) {
      this.syncTargetPath = path;
      this.saveSourceConfig();
    },

    setImportedDbPath(path: string) {
      this.importedDbPath = path;
      this.saveSourceConfig();
    },

    setActiveDbPath(path: string) {
      this.activeDbPath = path;
      this.lastReadyAt = new Date().toLocaleString("zh-CN", { hour12: false });
      this.saveSourceConfig();
    },

    async validateLocalDatabase(path: string): Promise<void> {
      await invoke("validate_local_database", { path });
    },

    async exportWideTable(params: ExportWideTableParams): Promise<void> {
      this.loading = true;
      this.error = null;
      this.progress = 0;
      this.progressMessage = "准备导出...";
      this.logs = [];
      this.exportedRows = 0;
      this.exportedPath = null;

      // 监听实时日志事件（后端已经包含了时间戳和格式化的日志）
      const unlisten = await listen<string>("query-log", (event) => {
        // 后端已经包含了时间戳，直接添加
        this.logs.push(event.payload);
      });

      try {
        this.updateProgress(10, "开始导出...");
        
        const rowCount = await invoke<number>("export_wide_table_direct", { params });
        
        this.exportedRows = rowCount;
        this.exportedPath = params.outputPath;
        this.progress = 100;
        this.progressMessage = `导出完成 (${rowCount} 条记录)`;
      } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error);
        this.error = errorMsg;
        this.progressMessage = "导出失败";
        // 错误信息通过后端日志事件已经发送，这里只更新UI状态
      } finally {
        // 取消事件监听
        unlisten();
        this.loading = false;
      }
    },

    async exportDemandResults(params: ExportDemandResultsParams): Promise<void> {
      this.loading = true;
      this.error = null;
      this.progress = 0;
      this.progressMessage = "准备导出...";
      this.logs = [];
      this.exportedRows = 0;
      this.exportedPath = null;

      // 监听实时日志事件（后端已经包含了时间戳和格式化的日志）
      const unlisten = await listen<string>("query-log", (event) => {
        // 后端已经包含了时间戳，直接添加
        this.logs.push(event.payload);
      });

      try {
        this.updateProgress(10, "开始导出...");
        
        const rowCount = await invoke<number>("export_demand_results_direct", { params });
        
        this.exportedRows = rowCount;
        this.exportedPath = params.outputPath;
        this.progress = 100;
        this.progressMessage = `导出完成 (${rowCount} 条记录)`;
      } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error);
        this.error = errorMsg;
        this.progressMessage = "导出失败";
        // 错误信息通过后端日志事件已经发送，这里只更新UI状态
      } finally {
        // 取消事件监听
        unlisten();
        this.loading = false;
      }
    },

    async executeQuery(params: QueryParams): Promise<void> {
      this.loading = true;
      this.error = null;
      this.progress = 0;
      this.progressMessage = "正在连接...";
      this.logs = [];

      // 监听实时日志事件（后端已经包含了时间戳和格式化的日志）
      const unlisten = await listen<string>("query-log", (event) => {
        // 后端已经包含了时间戳，直接添加
        this.logs.push(event.payload);
      });

      try {
        this.updateProgress(10, "开始查询...");
        
        const result = await invoke<QueryResult>("execute_query", { 
          params: {
            ...params,
            queryType: "wide_table",
          }
        });
        
        this.results = result;
        this.progress = 100;
        this.progressMessage = `查询完成 (${result.totalRows} 条记录)`;
      } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error);
        this.error = errorMsg;
        this.progressMessage = "查询失败";
        // 错误信息通过后端日志事件已经发送，这里只更新UI状态
      } finally {
        // 取消事件监听
        unlisten();
        this.loading = false;
      }
    },

    async syncDatabase(params: SyncDatabaseParams): Promise<void> {
      this.syncing = true;
      this.error = null;
      this.logs = [];
      this.syncProgress = 0;
      this.syncProgressMessage = "准备同步...";

      const unlistenLog = await listen<string>("query-log", (event) => {
        this.logs.push(event.payload);
      });

      const unlistenProgress = await listen<{
        downloaded: number;
        total: number;
        percent: number;
      }>("db-sync-progress", (event) => {
        const { downloaded, total, percent } = event.payload;
        this.syncProgress = percent;
        const mb = (n: number) => (n / 1024 / 1024).toFixed(2);
        this.syncProgressMessage = `${mb(downloaded)}MB / ${mb(total)}MB (${percent}%)`;
      });

      try {
        const syncedPath = await invoke<string>("sync_database", {
          dbPath: params.dbPath,
          targetPath: params.targetPath || this.syncTargetPath || null,
          startTime: params.startTime ?? null,
          endTime: params.endTime ?? null,
        });
        this.dbSynced = true;
        this.dbSyncTime = new Date().toLocaleTimeString("zh-CN", {
          hour12: false,
        });
        this.setActiveDbPath(syncedPath);
        this.syncProgress = 100;
        this.syncProgressMessage = "同步完成";
      } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error);
        this.error = errorMsg;
        this.syncProgressMessage = "同步失败";
      } finally {
        unlistenLog();
        unlistenProgress();
        this.syncing = false;
      }
    },

    async clearDbCache(): Promise<void> {
      try {
        await invoke("clear_db_cache");
        this.dbSynced = false;
        this.dbSyncTime = null;
        if (this.sourceMode === "remote_sync") {
          this.activeDbPath = "";
          this.saveSourceConfig();
        }
      } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error);
        this.error = errorMsg;
      }
    },

    updateProgress(progress: number, message: string) {
      this.progress = progress;
      this.progressMessage = message;
    },

    clearResults() {
      this.results = null;
      this.error = null;
      this.progress = 0;
      this.progressMessage = "";
      this.logs = [];
      this.exportedRows = 0;
      this.exportedPath = null;
      // dbSynced 和 dbSyncTime 不重置，跨查询保持同步状态
    },
  },
});

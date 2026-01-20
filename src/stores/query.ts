import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// 注意：时间格式化已由后端统一处理，前端不再需要格式化函数

export interface QueryParams {
  dbPath: string;
  startTime: number;
  endTime: number;
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
  }),

  actions: {
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
    },
  },
});

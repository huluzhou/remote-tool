import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";

export interface QueryParams {
  dbPath: string;
  startTime: number;
  endTime: number;
  deviceSn?: string;
  includeExt?: boolean;
  queryType: "device" | "command" | "wide_table";
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
  }),

  actions: {
    async executeQuery(params: QueryParams): Promise<void> {
      this.loading = true;
      this.error = null;
      this.progress = 0;
      this.progressMessage = "正在连接...";

      try {
        const result = await invoke<QueryResult>("execute_query", { params });
        this.results = result;
        this.progress = 100;
        this.progressMessage = "查询完成";
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        this.progressMessage = "查询失败";
      } finally {
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
    },
  },
});

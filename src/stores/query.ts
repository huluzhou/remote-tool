import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// 格式化时间为 GMT+8 时区
function formatGMT8Time(timestamp: number): string {
  const date = new Date(timestamp);
  // 获取 UTC 时间并加上 8 小时
  const utcTime = date.getTime() + date.getTimezoneOffset() * 60 * 1000;
  const beijingTime = new Date(utcTime + 8 * 60 * 60 * 1000);
  return beijingTime.toISOString().slice(0, 19).replace('T', ' ');
}

// 格式化时间为 GMT+8 时区（仅时间部分）
function formatGMT8TimeOnly(timestamp: number): string {
  const date = new Date(timestamp);
  // 获取 UTC 时间并加上 8 小时
  const utcTime = date.getTime() + date.getTimezoneOffset() * 60 * 1000;
  const beijingTime = new Date(utcTime + 8 * 60 * 60 * 1000);
  return beijingTime.toISOString().slice(11, 19);
}

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
  csvFilePath?: string; // 解压后的CSV文件路径，供导出时直接使用
}

export const useQueryStore = defineStore("query", {
  state: () => ({
    results: null as QueryResult | null,
    loading: false,
    error: null as string | null,
    progress: 0,
    progressMessage: "",
    logs: [] as string[],
    queryType: null as "device" | "command" | "wide_table" | null,
  }),

  actions: {
    async executeQuery(params: QueryParams): Promise<void> {
      this.loading = true;
      this.error = null;
      this.progress = 0;
      this.progressMessage = "正在连接...";
      this.logs = [];
      this.addLog("开始执行查询...");
      this.addLog(`查询类型: ${params.queryType}`);
      this.addLog(`数据库路径: ${params.dbPath}`);
      // 使用 GMT+8 时区格式化时间范围
      this.addLog(`时间范围: ${formatGMT8Time(params.startTime * 1000)} - ${formatGMT8Time(params.endTime * 1000)}`);

      // 监听实时日志事件
      const unlisten = await listen<string>("query-log", (event) => {
        // 后端已经包含了时间戳，直接添加
        this.logs.push(event.payload);
      });

      try {
        this.addLog("正在连接数据库...");
        this.updateProgress(10, "正在连接数据库...");
        
        this.addLog("正在执行SQL查询...");
        this.updateProgress(30, "正在执行SQL查询...");
        
        const result = await invoke<QueryResult>("execute_query", { params });
        
        this.addLog(`查询成功！共找到 ${result.totalRows} 条记录`);
        this.results = result;
        this.queryType = params.queryType; // 保存查询类型
        this.progress = 100;
        this.progressMessage = `查询完成 (${result.totalRows} 条记录)`;
        this.addLog("查询完成");
      } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error);
        this.error = errorMsg;
        this.progressMessage = "查询失败";
        this.addLog(`查询失败: ${errorMsg}`);
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

    addLog(message: string) {
      // 使用 GMT+8 时区格式化时间
      const timestamp = formatGMT8TimeOnly(Date.now());
      this.logs.push(`[${timestamp}] ${message}`);
    },

    clearResults() {
      this.results = null;
      this.error = null;
      this.progress = 0;
      this.progressMessage = "";
      this.logs = [];
      this.queryType = null;
    },
  },
});

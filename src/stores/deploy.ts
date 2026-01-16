import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface DeployFile {
  localPath?: string;      // 本地文件路径（用于上传）
  remotePath?: string;     // 远程文件路径（用于上传和下载）
  downloadPath?: string;   // 下载到本地的路径（用于下载）
}

export interface DeployConfig {
  files: DeployFile[];     // 文件列表
  useRoot: boolean;        // 是否使用root用户
  restartService: boolean; // 是否重启服务（上传默认true，下载默认false）
}

export interface DeployStatus {
  installed: boolean;
  serviceExists: boolean;
  serviceRunning: boolean;
  serviceEnabled: boolean;
}

export const useDeployStore = defineStore("deploy", {
  state: () => ({
    status: null as DeployStatus | null,
    deploying: false,
    logs: [] as string[],
    error: null as string | null,
  }),

  actions: {
    async checkStatus(): Promise<void> {
      this.error = null;
      try {
        const status = await invoke<DeployStatus>("check_deploy_status");
        this.status = status;
      } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error);
        this.error = errorMsg;
      }
    },

    async deploy(config: DeployConfig): Promise<{ success: boolean; error?: string }> {
      this.deploying = true;
      this.error = null;
      this.logs = [];

      // 监听实时日志事件
      const unlisten = await listen<string>("deploy-log", (event) => {
        this.logs.push(event.payload);
      });

      try {
        const result = await invoke<{ success: boolean; error?: string; logs: string[] }>(
          "deploy_application",
          { config }
        );

        // 如果事件监听没有接收到所有日志，使用返回的日志作为补充
        if (result.logs && result.logs.length > this.logs.length) {
          this.logs = result.logs;
        }

        if (!result.success) {
          this.error = result.error || "部署失败";
        }

        return result;
      } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error);
        this.error = errorMsg;
        this.logs.push(`错误: ${errorMsg}`);
        return { success: false, error: errorMsg };
      } finally {
        // 取消事件监听
        unlisten();
        this.deploying = false;
      }
    },

    addLog(message: string) {
      this.logs.push(message);
    },

    clearLogs() {
      this.logs = [];
      this.error = null;
    },
  },
});

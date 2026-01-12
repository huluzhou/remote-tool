import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface DeployConfig {
  binaryPath?: string;
  configPath?: string;
  topoPath?: string;
  uploadBinary?: boolean;
  uploadConfig: boolean;
  uploadTopo: boolean;
  useRoot: boolean;
  startService: boolean;
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
        console.log("[DEBUG] 开始检查部署状态...");
        const status = await invoke<DeployStatus>("check_deploy_status");
        console.log("[DEBUG] 状态检查结果:", {
          installed: status.installed,
          serviceExists: status.serviceExists,
          serviceRunning: status.serviceRunning,
          serviceEnabled: status.serviceEnabled,
        });
        this.status = status;
      } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error);
        console.error("[DEBUG] 状态检查失败:", errorMsg);
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

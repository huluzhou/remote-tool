import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";

export interface DeployConfig {
  binaryPath: string;
  configPath?: string;
  topoPath?: string;
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
      try {
        const status = await invoke<DeployStatus>("check_deploy_status");
        this.status = status;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
      }
    },

    async deploy(config: DeployConfig): Promise<{ success: boolean; error?: string }> {
      this.deploying = true;
      this.error = null;
      this.logs = ["开始部署..."];

      try {
        const result = await invoke<{ success: boolean; error?: string; logs: string[] }>(
          "deploy_application",
          { config }
        );

        this.logs = result.logs || [];
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

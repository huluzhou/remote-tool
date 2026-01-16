import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";

export interface SshConfig {
  host: string;
  port: number;
  username: string;
  password?: string;
  keyFile?: string;
}

export interface SshConnection {
  id: string;
  config: SshConfig;
  connected: boolean;
  lastConnected?: Date;
}

export const useSshStore = defineStore("ssh", {
  state: () => ({
    connections: [] as SshConnection[],
    currentConnectionId: null as string | null,
  }),

  getters: {
    currentConnection(): SshConnection | null {
      if (!this.currentConnectionId) return null;
      return (
        this.connections.find((c) => c.id === this.currentConnectionId) || null
      );
    },

    isConnected(): boolean {
      return this.currentConnection?.connected || false;
    },
  },

  actions: {
    async connect(config: SshConfig): Promise<{ success: boolean; error?: string }> {
      try {
        const result = await invoke<{ success: boolean; error?: string }>(
          "ssh_connect",
          { config }
        );

        if (result.success) {
          const connection: SshConnection = {
            id: `${config.host}:${config.port}`,
            config,
            connected: true,
            lastConnected: new Date(),
          };

          const existingIndex = this.connections.findIndex(
            (c) => c.id === connection.id
          );
          if (existingIndex >= 0) {
            this.connections[existingIndex] = connection;
          } else {
            this.connections.push(connection);
          }

          this.currentConnectionId = connection.id;
        }

        return result;
      } catch (error) {
        return {
          success: false,
          error: error instanceof Error ? error.message : String(error),
        };
      }
    },

    async disconnect(): Promise<void> {
      if (!this.currentConnectionId) return;

      try {
        await invoke("ssh_disconnect");
        const connection = this.currentConnection;
        if (connection) {
          connection.connected = false;
        }
        this.currentConnectionId = null;
      } catch (error) {
        console.error("断开连接失败:", error);
      }
    },

    setCurrentConnection(id: string | null) {
      this.currentConnectionId = id;
    },
  },
});

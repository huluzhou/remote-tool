<template>
  <div class="query-view">
    <SshConnection />
    <div v-if="sshStore.isConnected" class="query-container">
      <QueryForm @query="handleQuery" />
      <div v-if="queryStore.loading || queryStore.results" class="results-section">
        <div v-if="queryStore.loading" class="loading">
          <div class="progress-bar">
            <div
              class="progress-fill"
              :style="{ width: `${queryStore.progress}%` }"
            ></div>
          </div>
          <p>{{ queryStore.progressMessage }}</p>
        </div>
        <QueryResults
          v-if="queryStore.results"
          :results="queryStore.results"
          @export="handleExport"
        />
        <div v-if="queryStore.error" class="error">
          {{ queryStore.error }}
        </div>
      </div>
    </div>
    <div v-else class="not-connected">
      <p>请先连接SSH服务器</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useSshStore } from "../stores/ssh";
import { useQueryStore } from "../stores/query";
import SshConnection from "../components/SshConnection.vue";
import QueryForm from "../components/DataQuery/QueryForm.vue";
import QueryResults from "../components/DataQuery/QueryResults.vue";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";

const sshStore = useSshStore();
const queryStore = useQueryStore();

const handleQuery = async (params: any) => {
  await queryStore.executeQuery(params);
};

const handleExport = async (data: any) => {
  try {
    const filePath = await save({
      filters: [
        {
          name: "CSV",
          extensions: ["csv"],
        },
      ],
      defaultPath: `query_result_${Date.now()}.csv`,
    });

    if (filePath) {
      await invoke("export_to_csv", { data, filePath });
    }
  } catch (error) {
    console.error("导出失败:", error);
  }
};
</script>

<style scoped>
.query-view {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.query-container {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.results-section {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.loading {
  padding: 1rem;
  background-color: rgba(255, 255, 255, 0.05);
  border-radius: 8px;
}

.progress-bar {
  width: 100%;
  height: 8px;
  background-color: rgba(255, 255, 255, 0.1);
  border-radius: 4px;
  overflow: hidden;
  margin-bottom: 0.5rem;
}

.progress-fill {
  height: 100%;
  background-color: #646cff;
  transition: width 0.3s;
}

.error {
  padding: 1rem;
  background-color: rgba(244, 67, 54, 0.1);
  color: #f44336;
  border-radius: 8px;
}

.not-connected {
  padding: 2rem;
  text-align: center;
  color: rgba(255, 255, 255, 0.5);
}
</style>

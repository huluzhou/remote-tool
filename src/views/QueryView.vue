<template>
  <div class="query-view">
    <SshConnection />
    <div v-if="sshStore.isConnected" class="query-container">
      <QueryForm @query="handleQuery" />
      <div v-if="queryStore.loading || queryStore.results || queryStore.logs.length > 0" class="results-section">
        <!-- 查询日志区域 -->
        <div v-if="queryStore.loading || queryStore.logs.length > 0" class="query-logs">
          <h4>查询日志</h4>
          <div class="logs-container" ref="logsContainerRef">
            <div
              v-for="(log, index) in queryStore.logs"
              :key="index"
              class="log-entry"
            >
              {{ log }}
            </div>
            <div v-if="queryStore.loading" class="log-entry loading-indicator">
              <span class="spinner"></span>
              查询进行中...
            </div>
          </div>
        </div>
        
        <!-- 进度条 -->
        <div v-if="queryStore.loading" class="loading">
          <div class="progress-bar">
            <div
              class="progress-fill"
              :style="{ width: `${queryStore.progress}%` }"
            ></div>
          </div>
          <p>{{ queryStore.progressMessage }}</p>
        </div>
        
        <!-- 查询结果 -->
        <QueryResults
          v-if="queryStore.results"
          :results="queryStore.results"
          @export="handleExport"
        />
        
        <!-- 导出按钮（即使结果为空也显示） -->
        <div v-if="!queryStore.loading && queryStore.results && queryStore.results.totalRows === 0" class="export-section">
          <div class="no-results-message">
            <p>查询完成，但没有找到匹配的记录</p>
            <button @click="handleExportFromStore" class="export-btn">导出为CSV</button>
          </div>
        </div>
        
        <!-- 错误信息 -->
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
import { ref, watch, nextTick } from "vue";
import { useSshStore } from "../stores/ssh";
import { useQueryStore } from "../stores/query";
import SshConnection from "../components/SshConnection.vue";
import QueryForm from "../components/DataQuery/QueryForm.vue";
import QueryResults from "../components/DataQuery/QueryResults.vue";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { message } from "@tauri-apps/plugin-dialog";

const sshStore = useSshStore();
const queryStore = useQueryStore();
const logsContainerRef = ref<HTMLElement | null>(null);

// 自动滚动日志到底部
const scrollLogsToBottom = () => {
  nextTick(() => {
    if (logsContainerRef.value) {
      logsContainerRef.value.scrollTop = logsContainerRef.value.scrollHeight;
    }
  });
};

// 监听日志变化，自动滚动
watch(() => queryStore.logs, () => {
  scrollLogsToBottom();
}, { deep: true });

const handleQuery = async (params: any) => {
  await queryStore.executeQuery(params);
};

const handleExport = async (data: any) => {
  try {
    // 检查是否有数据
    if (!data || !data.rows || data.rows.length === 0) {
      await message("没有可导出的数据", {
        title: "错误",
        kind: "error",
      });
      return;
    }

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
      await invoke("export_to_csv", { 
        data: {
          columns: data.columns,
          rows: data.rows,
          totalRows: data.totalRows,
        },
        filePath,
        queryType: queryStore.queryType 
      });
      await message("导出成功", {
        title: "成功",
        kind: "info",
      });
    }
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : String(error);
    console.error("导出失败:", error);
    await message(`导出失败: ${errorMsg}`, {
      title: "错误",
      kind: "error",
    });
  }
};

const handleExportFromStore = async () => {
  if (!queryStore.results) {
    await message("没有可导出的数据", {
      title: "提示",
      kind: "warning",
    });
    return;
  }
  
  await handleExport(queryStore.results);
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

.query-logs {
  padding: 1rem;
  background-color: rgba(255, 255, 255, 0.05);
  border-radius: 8px;
  margin-bottom: 1rem;
}

.query-logs h4 {
  margin: 0 0 0.75rem 0;
  font-size: 1rem;
  font-weight: 600;
}

.logs-container {
  max-height: 200px;
  overflow-y: auto;
  font-family: 'Courier New', monospace;
  font-size: 0.875rem;
  background-color: rgba(0, 0, 0, 0.2);
  padding: 0.75rem;
  border-radius: 4px;
}

.log-entry {
  padding: 0.25rem 0;
  color: rgba(255, 255, 255, 0.8);
  line-height: 1.5;
}

.log-entry:last-child {
  margin-bottom: 0;
}

.loading-indicator {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  color: #646cff;
}

.spinner {
  width: 12px;
  height: 12px;
  border: 2px solid rgba(100, 108, 255, 0.3);
  border-top-color: #646cff;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.export-section {
  padding: 1rem;
  background-color: rgba(255, 255, 255, 0.05);
  border-radius: 8px;
}

.no-results-message {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1rem;
  padding: 1rem;
}

.no-results-message p {
  margin: 0;
  color: rgba(255, 255, 255, 0.7);
}

.export-btn {
  padding: 0.5rem 1rem;
  background-color: #4caf50;
  color: white;
  font-size: 0.875rem;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  transition: background-color 0.2s;
}

.export-btn:hover {
  background-color: #45a049;
}
</style>

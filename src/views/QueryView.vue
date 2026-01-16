<template>
  <div class="query-view">
    <SshConnection />
    <div v-if="sshStore.isConnected" class="query-container">
      <QueryForm @query="handleQuery" />
      <div v-if="queryStore.loading || queryStore.logs.length > 0 || queryStore.exportedPath || queryStore.error" class="results-section">
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
        
        <!-- 导出结果信息 -->
        <div v-if="!queryStore.loading && queryStore.exportedPath" class="export-result">
          <div class="success-message">
            <p>✓ 导出成功！</p>
            <p>共导出 {{ queryStore.exportedRows }} 条记录</p>
            <p class="file-path">文件路径: {{ queryStore.exportedPath }}</p>
          </div>
        </div>
        
        <!-- 导出失败或无数据 -->
        <div v-if="!queryStore.loading && !queryStore.exportedPath && queryStore.exportedRows === 0 && !queryStore.error" class="export-section">
          <div class="no-results-message">
            <p>导出完成，但没有找到匹配的记录</p>
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
import { save } from "@tauri-apps/plugin-dialog";

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
  // 先弹出文件保存对话框
  let filePath: string | null = null;
  try {
    filePath = await save({
      filters: [
        {
          name: "CSV",
          extensions: ["csv"],
        },
      ],
      defaultPath: `wide_table_${Date.now()}.csv`,
    });
  } catch (error) {
    console.error("保存对话框失败:", error);
    return;
  }

  if (!filePath) {
    // 用户取消了保存对话框
    return;
  }

  // 调用导出函数
  await queryStore.exportWideTable({
    dbPath: params.dbPath,
    startTime: params.startTime,
    endTime: params.endTime,
    outputPath: filePath,
  });
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

.export-result {
  padding: 1rem;
  background-color: rgba(76, 175, 80, 0.1);
  border-radius: 8px;
  border: 1px solid rgba(76, 175, 80, 0.3);
}

.success-message {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.success-message p {
  margin: 0;
  color: rgba(255, 255, 255, 0.9);
}

.success-message p:first-child {
  font-weight: 600;
  color: #4caf50;
  font-size: 1.1rem;
}

.file-path {
  font-size: 0.875rem;
  color: rgba(255, 255, 255, 0.7);
  word-break: break-all;
}
</style>

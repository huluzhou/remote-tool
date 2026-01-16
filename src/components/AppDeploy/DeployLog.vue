<template>
  <div class="deploy-log">
    <div class="log-header">
      <h3>部署日志</h3>
      <button @click="clearLogs" class="clear-btn">清空</button>
    </div>
    <div class="log-content" ref="logContainer">
      <div
        v-for="(log, index) in logs"
        :key="index"
        :class="['log-line', getLogClass(log)]"
      >
        {{ log }}
      </div>
      <div v-if="error" class="log-error">
        错误: {{ error }}
      </div>
      <div v-if="logs.length === 0 && !error" class="log-empty">
        暂无日志
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from "vue";

const props = defineProps<{
  logs: string[];
  error?: string | null;
}>();

const emit = defineEmits<{
  clear: [];
}>();

const logContainer = ref<HTMLElement>();

const getLogClass = (log: string): string => {
  if (log.includes("成功") || log.includes("✓")) return "log-success";
  if (log.includes("失败") || log.includes("错误") || log.includes("✗"))
    return "log-error";
  if (log.includes("警告")) return "log-warning";
  return "";
};

const clearLogs = () => {
  emit("clear");
};

watch(
  () => props.logs.length,
  () => {
    nextTick(() => {
      if (logContainer.value) {
        logContainer.value.scrollTop = logContainer.value.scrollHeight;
      }
    });
  }
);
</script>

<style scoped>
.deploy-log {
  display: flex;
  flex-direction: column;
  height: 400px;
  background-color: rgba(255, 255, 255, 0.05);
  border-radius: 8px;
  padding: 1.5rem;
}

.log-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1rem;
}

.log-header h3 {
  font-size: 1.25rem;
}

.clear-btn {
  padding: 0.5rem 1rem;
  background-color: rgba(255, 255, 255, 0.1);
  font-size: 0.875rem;
}

.log-content {
  flex: 1;
  overflow-y: auto;
  font-family: "Courier New", monospace;
  font-size: 0.875rem;
  line-height: 1.5;
  background-color: rgba(0, 0, 0, 0.3);
  padding: 1rem;
  border-radius: 4px;
}

.log-line {
  margin-bottom: 0.25rem;
  word-break: break-all;
}

.log-line.log-success {
  color: #4caf50;
}

.log-line.log-error {
  color: #f44336;
}

.log-line.log-warning {
  color: #ff9800;
}

.log-error {
  color: #f44336;
  font-weight: 600;
  margin-top: 0.5rem;
}

.log-empty {
  color: rgba(255, 255, 255, 0.5);
  text-align: center;
  padding: 2rem;
}
</style>

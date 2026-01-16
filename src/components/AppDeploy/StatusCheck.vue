<template>
  <div class="status-check">
    <h3>部署状态</h3>
    <button @click="handleCheck" class="check-btn" :disabled="checking">
      {{ checking ? "检查中..." : "检查状态" }}
    </button>
    <div v-if="status" class="status-info">
      <div class="status-item">
        <span class="status-label">已安装:</span>
        <span :class="['status-value', status.installed ? 'success' : 'error']">
          {{ status.installed ? "是" : "否" }}
        </span>
        <span class="status-path" v-if="!status.installed">(路径: /opt/analysis/bin/analysis-collector)</span>
      </div>
      <div class="status-item">
        <span class="status-label">服务文件存在:</span>
        <span
          :class="['status-value', status.serviceExists ? 'success' : 'error']"
        >
          {{ status.serviceExists ? "是" : "否" }}
        </span>
        <span class="status-path" v-if="!status.serviceExists">(路径: /etc/systemd/system/analysis-collector.service)</span>
      </div>
      <div class="status-item">
        <span class="status-label">服务运行中:</span>
        <span
          :class="['status-value', status.serviceRunning ? 'success' : 'error']"
        >
          {{ status.serviceRunning ? "是" : "否" }}
        </span>
      </div>
      <div class="status-item">
        <span class="status-label">服务已启用:</span>
        <span
          :class="['status-value', status.serviceEnabled ? 'success' : 'error']"
        >
          {{ status.serviceEnabled ? "是" : "否" }}
        </span>
      </div>
      <div class="debug-info" v-if="showDebug">
        <details>
          <summary>调试信息（点击展开）</summary>
          <div class="debug-content">
            <p>请打开浏览器开发者工具（F12）查看控制台中的详细调试日志</p>
            <p>调试日志包含：</p>
            <ul>
              <li>执行的命令</li>
              <li>命令的退出码</li>
              <li>stdout 输出</li>
              <li>stderr 输出</li>
              <li>最终判断结果</li>
            </ul>
          </div>
        </details>
      </div>
    </div>
    <div v-else class="no-status">
      <p>点击"检查状态"查看部署状态</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from "vue";
import { useDeployStore } from "../../stores/deploy";

const emit = defineEmits<{
  check: [];
}>();

const deployStore = useDeployStore();
const checking = ref(false);
const showDebug = ref(false);

const status = computed(() => deployStore.status);

const handleCheck = async () => {
  checking.value = true;
  console.log("[DEBUG] 用户点击检查状态按钮");
  emit("check");
  // 等待状态检查完成
  await new Promise(resolve => setTimeout(resolve, 1500));
  checking.value = false;
  // 如果状态异常，自动显示调试信息
  if (status.value) {
    const hasIssue = !status.value.installed || !status.value.serviceExists || 
                     (status.value.serviceExists && !status.value.serviceRunning);
    if (hasIssue) {
      showDebug.value = true;
    }
  }
};
</script>

<style scoped>
.status-check {
  padding: 1.5rem;
  background-color: rgba(255, 255, 255, 0.05);
  border-radius: 8px;
}

.status-check h3 {
  margin-bottom: 1rem;
  font-size: 1.25rem;
}

.check-btn {
  width: 100%;
  padding: 0.75rem;
  background-color: #646cff;
  color: white;
  font-size: 1rem;
  font-weight: 500;
  margin-bottom: 1rem;
}

.check-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.status-info {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.status-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.5rem;
  background-color: rgba(255, 255, 255, 0.05);
  border-radius: 4px;
}

.status-label {
  font-size: 0.875rem;
}

.status-value {
  font-weight: 600;
  font-size: 0.875rem;
}

.status-value.success {
  color: #4caf50;
}

.status-value.error {
  color: #f44336;
}

.no-status {
  padding: 2rem;
  text-align: center;
  color: rgba(255, 255, 255, 0.5);
}

.status-path {
  font-size: 0.75rem;
  color: rgba(255, 255, 255, 0.6);
  margin-left: 0.5rem;
  font-family: monospace;
}

.debug-info {
  margin-top: 1rem;
  padding: 1rem;
  background-color: rgba(0, 0, 0, 0.3);
  border-radius: 4px;
  font-size: 0.875rem;
}

.debug-info summary {
  cursor: pointer;
  color: rgba(255, 255, 255, 0.8);
  font-weight: 500;
  margin-bottom: 0.5rem;
}

.debug-info summary:hover {
  color: rgba(255, 255, 255, 1);
}

.debug-content {
  margin-top: 0.5rem;
  padding: 0.5rem;
  color: rgba(255, 255, 255, 0.7);
  line-height: 1.6;
}

.debug-content ul {
  margin: 0.5rem 0;
  padding-left: 1.5rem;
}

.debug-content li {
  margin: 0.25rem 0;
}
</style>

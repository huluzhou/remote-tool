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
      </div>
      <div class="status-item">
        <span class="status-label">服务文件存在:</span>
        <span
          :class="['status-value', status.serviceExists ? 'success' : 'error']"
        >
          {{ status.serviceExists ? "是" : "否" }}
        </span>
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

const status = computed(() => deployStore.status);

const handleCheck = async () => {
  checking.value = true;
  emit("check");
  setTimeout(() => {
    checking.value = false;
  }, 1000);
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
</style>

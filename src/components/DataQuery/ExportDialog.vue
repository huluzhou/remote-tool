<template>
  <div v-if="show" class="export-dialog-overlay" @click="handleClose">
    <div class="export-dialog" @click.stop>
      <h3>导出CSV</h3>
      <div class="dialog-content">
        <p>准备导出 {{ totalRows }} 条记录</p>
        <div class="options">
          <label>
            <input v-model="options.includeHeaders" type="checkbox" />
            包含表头
          </label>
          <label>
            <input v-model="options.formatTimestamps" type="checkbox" />
            格式化时间戳
          </label>
        </div>
      </div>
      <div class="dialog-actions">
        <button @click="handleClose" class="cancel-btn">取消</button>
        <button @click="handleExport" class="export-btn">导出</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";

const props = defineProps<{
  show: boolean;
  totalRows: number;
}>();

const emit = defineEmits<{
  export: [options: any];
  close: [];
}>();

const options = ref({
  includeHeaders: true,
  formatTimestamps: true,
});

const handleExport = () => {
  emit("export", options.value);
};

const handleClose = () => {
  emit("close");
};
</script>

<style scoped>
.export-dialog-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.export-dialog {
  background-color: #1a1a1a;
  border-radius: 8px;
  padding: 1.5rem;
  min-width: 400px;
  max-width: 90vw;
}

.export-dialog h3 {
  margin-bottom: 1rem;
  font-size: 1.25rem;
}

.dialog-content {
  margin-bottom: 1.5rem;
}

.options {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  margin-top: 1rem;
}

.options label {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  cursor: pointer;
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 1rem;
}

.cancel-btn,
.export-btn {
  padding: 0.5rem 1rem;
  font-size: 0.875rem;
}

.export-btn {
  background-color: #4caf50;
  color: white;
}
</style>

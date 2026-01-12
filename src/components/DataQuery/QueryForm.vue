<template>
  <div class="query-form">
    <h3>查询配置</h3>
    <form @submit.prevent="handleSubmit">
      <div class="form-group">
        <label>数据库路径:</label>
        <input
          v-model="formData.dbPath"
          type="text"
          placeholder="/mnt/analysis/data/device_data.db"
          required
        />
      </div>
      <div class="form-group">
        <label>查询类型:</label>
        <div class="radio-group">
          <label>
            <input
              v-model="formData.queryType"
              type="radio"
              value="device"
            />
            设备数据
          </label>
          <label>
            <input
              v-model="formData.queryType"
              type="radio"
              value="command"
            />
            指令数据
          </label>
          <label>
            <input
              v-model="formData.queryType"
              type="radio"
              value="wide_table"
            />
            宽表
          </label>
        </div>
      </div>
      <div v-if="formData.queryType !== 'wide_table'" class="form-group">
        <label>设备序列号:</label>
        <input
          v-model="formData.deviceSn"
          type="text"
          placeholder="可选"
        />
      </div>
      <div class="form-group">
        <label>开始时间:</label>
        <div class="time-input-group">
          <input
            v-model="startTimeInput"
            type="text"
            placeholder="时间戳或日期"
            required
          />
          <div class="quick-buttons">
            <button type="button" @click="setTimeRange('today')">今天</button>
            <button type="button" @click="setTimeRange('yesterday')">昨天</button>
            <button type="button" @click="setTimeRange('7days')">最近7天</button>
          </div>
        </div>
      </div>
      <div class="form-group">
        <label>结束时间:</label>
        <div class="time-input-group">
          <input
            v-model="endTimeInput"
            type="text"
            placeholder="时间戳或日期"
            required
          />
          <button type="button" @click="setEndTimeNow">现在</button>
        </div>
      </div>
      <div
        v-if="formData.queryType !== 'command'"
        class="form-group"
      >
        <label>
          <input
            v-model="formData.includeExt"
            type="checkbox"
          />
          包含扩展表数据
        </label>
      </div>
      <button type="submit" class="submit-btn" :disabled="loading">
        {{ loading ? "查询中..." : "执行查询" }}
      </button>
    </form>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { useQueryStore } from "../../stores/query";

const emit = defineEmits<{
  query: [params: any];
}>();

const queryStore = useQueryStore();
const loading = computed(() => queryStore.loading);

const formData = ref({
  dbPath: "/mnt/analysis/data/device_data.db",
  queryType: "wide_table" as "device" | "command" | "wide_table",
  deviceSn: "",
  includeExt: true,
});

const startTimeInput = ref("");
const endTimeInput = ref("");

const parseTime = (input: string): number | null => {
  const trimmed = input.trim();
  if (!trimmed) return null;

  // 如果是纯数字，当作时间戳
  if (/^\d+$/.test(trimmed)) {
    return parseInt(trimmed);
  }

  // 尝试解析日期
  const date = new Date(trimmed);
  if (!isNaN(date.getTime())) {
    return Math.floor(date.getTime() / 1000);
  }

  return null;
};

const setTimeRange = (type: string) => {
  const now = new Date();
  let start: Date;

  switch (type) {
    case "today":
      start = new Date(now);
      start.setHours(0, 0, 0, 0);
      startTimeInput.value = Math.floor(start.getTime() / 1000).toString();
      endTimeInput.value = Math.floor(now.getTime() / 1000).toString();
      break;
    case "yesterday":
      start = new Date(now);
      start.setDate(start.getDate() - 1);
      start.setHours(0, 0, 0, 0);
      const end = new Date(start);
      end.setHours(23, 59, 59, 999);
      startTimeInput.value = Math.floor(start.getTime() / 1000).toString();
      endTimeInput.value = Math.floor(end.getTime() / 1000).toString();
      break;
    case "7days":
      start = new Date(now);
      start.setDate(start.getDate() - 7);
      startTimeInput.value = Math.floor(start.getTime() / 1000).toString();
      endTimeInput.value = Math.floor(now.getTime() / 1000).toString();
      break;
  }
};

const setEndTimeNow = () => {
  endTimeInput.value = Math.floor(new Date().getTime() / 1000).toString();
};

const handleSubmit = () => {
  if (loading.value) {
    return; // 如果正在查询，不允许再次点击
  }

  const startTime = parseTime(startTimeInput.value);
  const endTime = parseTime(endTimeInput.value);

  if (!startTime || !endTime) {
    alert("请输入有效的时间范围");
    return;
  }

  const params = {
    dbPath: formData.value.dbPath,
    queryType: formData.value.queryType,
    startTime,
    endTime,
    deviceSn: formData.value.deviceSn || undefined,
    includeExt: formData.value.includeExt,
  };

  emit("query", params);
};
</script>

<style scoped>
.query-form {
  padding: 1.5rem;
  background-color: rgba(255, 255, 255, 0.05);
  border-radius: 8px;
}

.query-form h3 {
  margin-bottom: 1rem;
  font-size: 1.25rem;
}

.form-group {
  margin-bottom: 1rem;
}

.form-group label {
  display: block;
  margin-bottom: 0.5rem;
  font-size: 0.875rem;
  font-weight: 500;
}

.form-group input[type="text"] {
  width: 100%;
  padding: 0.5rem;
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 4px;
  background-color: rgba(255, 255, 255, 0.05);
  color: inherit;
  font-size: 0.875rem;
}

.radio-group {
  display: flex;
  gap: 1rem;
}

.radio-group label {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  cursor: pointer;
}

.time-input-group {
  display: flex;
  gap: 0.5rem;
  align-items: center;
}

.time-input-group input {
  flex: 1;
}

.quick-buttons {
  display: flex;
  gap: 0.5rem;
}

.quick-buttons button {
  padding: 0.5rem 1rem;
  font-size: 0.875rem;
  background-color: rgba(255, 255, 255, 0.1);
}

.submit-btn {
  width: 100%;
  padding: 0.75rem;
  background-color: #646cff;
  color: white;
  font-size: 1rem;
  font-weight: 500;
  margin-top: 1rem;
}

.submit-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>

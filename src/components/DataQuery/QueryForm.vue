<template>
  <div class="query-form">
    <h3>{{ queryType === 'wide_table' ? '导出宽表数据' : '导出需量数据' }}</h3>
    <form @submit.prevent="handleSubmit">
      <div class="form-group">
        <label>数据库来源:</label>
        <div class="radio-group">
          <label>
            <input
              type="radio"
              value="remote_sync"
              v-model="sourceMode"
            />
            远程同步（默认）
          </label>
          <label>
            <input
              type="radio"
              value="local_import"
              v-model="sourceMode"
            />
            本地导入（备用）
          </label>
        </div>
      </div>
      <div class="form-group">
        <label>查询类型:</label>
        <div class="radio-group">
          <label>
            <input
              type="radio"
              value="wide_table"
              v-model="queryType"
            />
            宽表查询
          </label>
          <label>
            <input
              type="radio"
              value="demand"
              v-model="queryType"
            />
            需量查询
          </label>
        </div>
      </div>
      <div v-if="sourceMode === 'remote_sync'" class="form-group">
        <label>数据库路径:</label>
        <input
          v-model="remoteDbPath"
          type="text"
          placeholder="/mnt/analysis/data/device_data.db"
          required
        />
      </div>
      <div v-if="sourceMode === 'remote_sync'" class="form-group">
        <label>同步落盘路径:</label>
        <div class="time-input-group">
          <input
            v-model="syncTargetPath"
            type="text"
            placeholder="请选择本地数据库保存路径（*.db）"
            required
          />
          <button type="button" @click="pickSyncTargetPath">选择路径</button>
        </div>
      </div>
      <div v-if="sourceMode === 'remote_sync'" class="form-group sync-group">
        <div class="sync-controls">
          <button
            type="button"
            class="sync-btn"
            :disabled="syncing || !sshConnected"
            @click="handleSync"
          >
            {{ syncing ? '同步中...' : '同步数据库' }}
          </button>
          <span class="sync-status" :class="{ synced: queryStore.dbSynced }">
            {{ queryStore.dbSynced ? `已同步 (${queryStore.dbSyncTime})` : '未同步' }}
          </span>
        </div>
        <div v-if="syncing" class="sync-progress">
          <div class="progress-bar">
            <div
              class="progress-fill"
              :style="{ width: `${queryStore.syncProgress}%` }"
            />
          </div>
          <span class="progress-text">{{ queryStore.syncProgressMessage }}</span>
        </div>
      </div>
      <div v-if="sourceMode === 'local_import'" class="form-group">
        <label>本地数据库文件:</label>
        <div class="time-input-group">
          <input
            :value="queryStore.importedDbPath"
            type="text"
            placeholder="请选择已下载数据库文件（*.db）"
            readonly
          />
          <button type="button" @click="importLocalDatabase">导入数据库</button>
        </div>
      </div>
      <div v-if="queryStore.activeDbPath" class="form-group active-db-info">
        <p>当前活动数据库: {{ queryStore.activeDbPath }}</p>
        <p v-if="queryStore.lastReadyAt">最近可用时间: {{ queryStore.lastReadyAt }}</p>
      </div>
      <div class="form-group">
        <label>开始时间:</label>
        <div class="time-input-group">
          <input
            v-model="startDateTimeText"
            type="text"
            placeholder="YYYY-MM-DD HH:mm:ss"
            required
          />
          <div class="quick-buttons">
            <button type="button" @click="setTimeRange('today')">今天</button>
            <button type="button" @click="setTimeRange('yesterday')">昨天</button>
            <button type="button" @click="setTimeRange('7days')">最近7天</button>
            <button type="button" @click="setTimeRange('30days')">最近30天</button>
            <button type="button" @click="setTimeRange('thisMonth')">本月</button>
          </div>
        </div>
      </div>
      <div class="form-group">
        <label>结束时间:</label>
        <div class="time-input-group">
          <input
            v-model="endDateTimeText"
            type="text"
            placeholder="YYYY-MM-DD HH:mm:ss"
            required
          />
          <button type="button" @click="setEndTimeNow">现在</button>
        </div>
      </div>
      <button type="submit" class="submit-btn" :disabled="loading">
        {{ loading ? "导出中..." : "导出CSV" }}
      </button>
    </form>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useQueryStore, type SourceMode } from "../../stores/query";
import { useSshStore } from "../../stores/ssh";
import { open, save } from "@tauri-apps/plugin-dialog";

const emit = defineEmits<{
  query: [params: any];
}>();

const queryStore = useQueryStore();
const sshStore = useSshStore();
const loading = computed(() => queryStore.loading);
const syncing = computed(() => queryStore.syncing);
const sshConnected = computed(() => sshStore.isConnected);
const sourceMode = computed({
  get: () => queryStore.sourceMode,
  set: (value: SourceMode) => queryStore.setSourceMode(value),
});
const remoteDbPath = computed({
  get: () => queryStore.remoteDbPath,
  set: (value) => queryStore.setRemoteDbPath(value),
});
const syncTargetPath = computed({
  get: () => queryStore.syncTargetPath,
  set: (value) => queryStore.setSyncTargetPath(value),
});

onMounted(async () => {
  queryStore.loadSourceConfig();
  if (queryStore.sourceMode === "remote_sync" && queryStore.activeDbPath) {
    try {
      await queryStore.validateLocalDatabase(queryStore.activeDbPath);
      queryStore.dbSynced = true;
    } catch {
      queryStore.dbSynced = false;
    }
  }
});

const handleSync = () => {
  if (syncing.value || !sshConnected.value) return;
  if (!remoteDbPath.value.trim()) {
    alert("请先填写远程数据库路径");
    return;
  }
  if (!syncTargetPath.value.trim()) {
    alert("请先设置同步落盘路径");
    return;
  }

  const startTime = parseDateTimeToSeconds(startDateTimeText.value);
  const endTime = parseDateTimeToSeconds(endDateTimeText.value);
  if (!startTime || !endTime) {
    alert("请先输入有效的开始/结束时间，再按时间范围同步数据库");
    return;
  }
  if (startTime > endTime) {
    alert("开始时间不能晚于结束时间");
    return;
  }

  queryStore.syncDatabase({
    dbPath: remoteDbPath.value,
    targetPath: syncTargetPath.value,
    startTime,
    endTime,
  });
};

const queryType = ref<"wide_table" | "demand">("wide_table");

const startDateTimeText = ref("");
const endDateTimeText = ref("");

const pickSyncTargetPath = async () => {
  const selected = await save({
    filters: [
      {
        name: "SQLite",
        extensions: ["db"],
      },
    ],
    defaultPath: queryStore.syncTargetPath || "device_data.db",
  });

  if (!selected) return;
  queryStore.setSyncTargetPath(selected);
};

const importLocalDatabase = async () => {
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [
      {
        name: "SQLite",
        extensions: ["db", "sqlite", "sqlite3"],
      },
    ],
  });

  if (!selected || Array.isArray(selected)) return;

  try {
    await queryStore.validateLocalDatabase(selected);
    queryStore.setImportedDbPath(selected);
    queryStore.setActiveDbPath(selected);
    queryStore.setSourceMode("local_import");
  } catch (error) {
    const msg = error instanceof Error ? error.message : String(error);
    alert(`导入数据库失败: ${msg}`);
  }
};

const pad2 = (n: number): string => String(n).padStart(2, "0");

const formatDateTimeText = (date: Date): string => {
  return `${date.getFullYear()}-${pad2(date.getMonth() + 1)}-${pad2(date.getDate())} ${pad2(date.getHours())}:${pad2(date.getMinutes())}:${pad2(date.getSeconds())}`;
};

const parseDateTimeToSeconds = (input: string): number | null => {
  const value = input.trim();
  if (!value) return null;

  if (/^\d+$/.test(value)) {
    const ts = Number(value);
    if (!Number.isFinite(ts)) return null;
    return value.length >= 13 ? Math.floor(ts / 1000) : ts;
  }

  let normalized = value.replace(/\//g, "-");
  if (/^\d{4}-\d{2}-\d{2}\s\d{2}:\d{2}$/.test(normalized)) {
    normalized += ":00";
  }
  normalized = normalized.replace(" ", "T");
  const date = new Date(normalized);
  if (isNaN(date.getTime())) return null;
  return Math.floor(date.getTime() / 1000);
};

const setTimeRange = (type: string) => {
  const now = new Date();
  let start = new Date(now);
  let end = new Date(now);

  switch (type) {
    case "today":
      start.setHours(0, 0, 0, 0);
      break;
    case "yesterday":
      start.setDate(start.getDate() - 1);
      start.setHours(0, 0, 0, 0);
      end = new Date(start);
      end.setHours(23, 59, 59, 999);
      break;
    case "7days":
      start.setDate(start.getDate() - 7);
      break;
    case "30days":
      start.setDate(start.getDate() - 30);
      break;
    case "thisMonth":
      start = new Date(now.getFullYear(), now.getMonth(), 1, 0, 0, 0);
      break;
  }

  startDateTimeText.value = formatDateTimeText(start);
  endDateTimeText.value = formatDateTimeText(end);
};

const setEndTimeNow = () => {
  endDateTimeText.value = formatDateTimeText(new Date());
};

const handleSubmit = async () => {
  if (loading.value) {
    return; // 如果正在查询，不允许再次点击
  }

  const startTime = parseDateTimeToSeconds(startDateTimeText.value);
  const endTime = parseDateTimeToSeconds(endDateTimeText.value);

  if (!startTime || !endTime) {
    alert("请输入有效的时间范围");
    return;
  }
  if (startTime > endTime) {
    alert("开始时间不能晚于结束时间");
    return;
  }

  const activeDbPath = queryStore.activeDbPath;
  if (!activeDbPath) {
    alert("请先同步数据库或导入本地数据库文件");
    return;
  }

  try {
    await queryStore.validateLocalDatabase(activeDbPath);
    const params = {
      queryType: queryType.value,
      dbPath: activeDbPath,
      startTime,
      endTime,
    };
    emit("query", params);
  } catch (error) {
    const msg = error instanceof Error ? error.message : String(error);
    alert(`当前数据库不可用，请重新同步或导入。${msg}`);
  }
};

// 初始化默认时间范围：最近7天
setTimeRange("7days");
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

.form-group input[type="text"],
.form-group input[type="date"],
.form-group input[type="time"] {
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
  margin-bottom: 0;
}

.radio-group input[type="radio"] {
  margin: 0;
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

.sync-group {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.sync-controls {
  display: flex;
  align-items: center;
  gap: 1rem;
}

.sync-progress {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.progress-bar {
  height: 6px;
  background-color: rgba(255, 255, 255, 0.1);
  border-radius: 3px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background-color: #4caf50;
  border-radius: 3px;
  transition: width 0.2s ease;
}

.progress-text {
  font-size: 0.75rem;
  color: rgba(255, 255, 255, 0.6);
}

.sync-btn {
  padding: 0.5rem 1.25rem;
  font-size: 0.875rem;
  font-weight: 500;
  color: #4caf50;
  background-color: rgba(76, 175, 80, 0.1);
  border: 1px solid rgba(76, 175, 80, 0.4);
  border-radius: 4px;
  cursor: pointer;
  transition: background-color 0.2s, border-color 0.2s;
  white-space: nowrap;
}

.sync-btn:hover:not(:disabled) {
  background-color: rgba(76, 175, 80, 0.2);
  border-color: rgba(76, 175, 80, 0.6);
}

.sync-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.sync-status {
  font-size: 0.8rem;
  color: rgba(255, 255, 255, 0.45);
}

.sync-status.synced {
  color: #4caf50;
}

.active-db-info {
  padding: 0.75rem;
  background-color: rgba(100, 108, 255, 0.1);
  border: 1px solid rgba(100, 108, 255, 0.3);
  border-radius: 6px;
}

.active-db-info p {
  margin: 0.25rem 0;
  word-break: break-all;
  font-size: 0.85rem;
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

<template>
  <div class="query-results">
    <div class="results-header">
      <h3>查询结果 ({{ results.totalRows }} 条)</h3>
      <button @click="handleExport" class="export-btn">导出为CSV</button>
    </div>
    <div class="table-container">
      <table class="results-table">
        <thead>
          <tr>
            <th v-for="column in results.columns" :key="column">
              {{ column }}
            </th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(row, index) in displayedRows" :key="index">
            <td v-for="column in results.columns" :key="column">
              {{ formatValue(row[column]) }}
            </td>
          </tr>
        </tbody>
      </table>
    </div>
    <div v-if="results.rows.length > pageSize" class="pagination">
      <button
        @click="currentPage--"
        :disabled="currentPage === 1"
        class="page-btn"
      >
        上一页
      </button>
      <span class="page-info">
        第 {{ currentPage }} / {{ totalPages }} 页
      </span>
      <button
        @click="currentPage++"
        :disabled="currentPage === totalPages"
        class="page-btn"
      >
        下一页
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from "vue";
import type { QueryResult } from "../../stores/query";

const props = defineProps<{
  results: QueryResult;
}>();

const emit = defineEmits<{
  export: [data: QueryResult];
}>();

const pageSize = 100;
const currentPage = ref(1);

const totalPages = computed(() =>
  Math.ceil(props.results.rows.length / pageSize)
);

const displayedRows = computed(() => {
  const start = (currentPage.value - 1) * pageSize;
  const end = start + pageSize;
  return props.results.rows.slice(start, end);
});

const formatValue = (value: any): string => {
  if (value === null || value === undefined) return "";
  if (typeof value === "object") return JSON.stringify(value);
  return String(value);
};

const handleExport = () => {
  emit("export", props.results);
};
</script>

<style scoped>
.query-results {
  display: flex;
  flex-direction: column;
  height: 100%;
  background-color: rgba(255, 255, 255, 0.05);
  border-radius: 8px;
  padding: 1.5rem;
}

.results-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1rem;
}

.results-header h3 {
  font-size: 1.25rem;
}

.export-btn {
  padding: 0.5rem 1rem;
  background-color: #4caf50;
  color: white;
  font-size: 0.875rem;
}

.table-container {
  flex: 1;
  overflow: auto;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 4px;
}

.results-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.875rem;
}

.results-table th {
  background-color: rgba(255, 255, 255, 0.1);
  padding: 0.75rem;
  text-align: left;
  font-weight: 600;
  position: sticky;
  top: 0;
  z-index: 1;
}

.results-table td {
  padding: 0.5rem 0.75rem;
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
}

.results-table tr:hover {
  background-color: rgba(255, 255, 255, 0.02);
}

.pagination {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 1rem;
  margin-top: 1rem;
}

.page-btn {
  padding: 0.5rem 1rem;
  background-color: rgba(255, 255, 255, 0.1);
  font-size: 0.875rem;
}

.page-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.page-info {
  font-size: 0.875rem;
}
</style>

<template>
  <div class="app-container">
    <header class="app-header">
      <h1>Remote Tool</h1>
      <div class="header-actions">
        <button @click="checkUpdate" v-if="updateAvailable" class="update-btn">
          有可用更新
        </button>
      </div>
    </header>
    <main class="app-main">
      <nav class="tabs">
        <button
          :class="['tab', { active: activeTab === 'query' }]"
          @click="activeTab = 'query'"
        >
          数据查询
        </button>
        <button
          :class="['tab', { active: activeTab === 'deploy' }]"
          @click="activeTab = 'deploy'"
        >
          应用部署
        </button>
      </nav>
      <div class="content">
        <QueryView v-if="activeTab === 'query'" />
        <DeployView v-if="activeTab === 'deploy'" />
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { check, install } from "@tauri-apps/plugin-updater";
import QueryView from "./views/QueryView.vue";
import DeployView from "./views/DeployView.vue";

const activeTab = ref<"query" | "deploy">("query");
const updateAvailable = ref(false);

onMounted(async () => {
  try {
    const update = await check();
    if (update?.available) {
      updateAvailable.value = true;
    }
  } catch (error) {
    console.error("检查更新失败:", error);
  }
});

const checkUpdate = async () => {
  try {
    const update = await check();
    if (update?.available) {
      await update.downloadAndInstall();
      await install();
    }
  } catch (error) {
    console.error("更新失败:", error);
  }
};
</script>

<style scoped>
.app-container {
  display: flex;
  flex-direction: column;
  height: 100vh;
  width: 100%;
}

.app-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 1rem 1.5rem;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  background-color: rgba(0, 0, 0, 0.2);
}

.app-header h1 {
  font-size: 1.5rem;
  font-weight: 600;
}

.header-actions {
  display: flex;
  gap: 0.5rem;
}

.update-btn {
  background-color: #646cff;
  color: white;
  padding: 0.5rem 1rem;
  font-size: 0.875rem;
}

.app-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.tabs {
  display: flex;
  gap: 0.5rem;
  padding: 1rem 1.5rem;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  background-color: rgba(0, 0, 0, 0.1);
}

.tab {
  padding: 0.5rem 1.5rem;
  background: transparent;
  border: none;
  border-bottom: 2px solid transparent;
  cursor: pointer;
  transition: all 0.2s;
}

.tab:hover {
  background-color: rgba(255, 255, 255, 0.05);
}

.tab.active {
  border-bottom-color: #646cff;
  color: #646cff;
}

.content {
  flex: 1;
  overflow: auto;
  padding: 1.5rem;
}
</style>

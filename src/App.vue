<template>
  <div class="app-container">
    <header class="app-header">
      <h1>Remote Tool</h1>
      <div class="header-actions">
        <button 
          @click="checkUpdate" 
          class="update-btn" 
          :class="{ 'update-available': updateAvailable }"
          :disabled="checkingUpdate"
        >
          {{ checkingUpdate ? '检查中...' : (updateAvailable ? '有可用更新' : '检查更新') }}
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
import { check } from "@tauri-apps/plugin-updater";
import QueryView from "./views/QueryView.vue";
import DeployView from "./views/DeployView.vue";

const activeTab = ref<"query" | "deploy">("query");
const updateAvailable = ref(false);
const checkingUpdate = ref(false);

// 应用启动时自动检查更新
onMounted(async () => {
  await checkForUpdates();
});

// 检查更新（不自动安装）
const checkForUpdates = async () => {
  try {
    const update = await check();
    if (update?.available) {
      updateAvailable.value = true;
      console.log(`发现新版本: ${update.version}，当前版本: ${update.currentVersion}`);
    } else {
      updateAvailable.value = false;
      console.log("已是最新版本");
    }
  } catch (error) {
    console.error("检查更新失败:", error);
    updateAvailable.value = false;
  }
};

// 手动检查更新并安装
const checkUpdate = async () => {
  if (checkingUpdate.value) return;
  
  checkingUpdate.value = true;
  try {
    const update = await check();
    if (update?.available) {
      updateAvailable.value = true;
      // 如果配置了 dialog: true，Tauri 会自动显示更新对话框
      // 这里我们也可以直接下载并安装
      try {
        await update.downloadAndInstall();
        // downloadAndInstall() 已经包含了安装功能，会自动重启应用
      } catch (installError) {
        // 安装失败时显示错误
        const errorMsg = installError instanceof Error ? installError.message : String(installError);
        alert(`下载/安装更新失败: ${errorMsg}`);
      }
    } else {
      // 没有更新时，显示提示
      updateAvailable.value = false;
      alert("已是最新版本");
    }
  } catch (error) {
    // 检查更新失败
    const errorMsg = error instanceof Error ? error.message : String(error);
    console.error("检查更新失败:", error);
    
    // 提供更友好的错误提示
    if (errorMsg.includes("404") || errorMsg.includes("Not Found")) {
      alert("检查更新失败：未找到更新服务器或当前版本信息");
    } else if (errorMsg.includes("network") || errorMsg.includes("request")) {
      alert("检查更新失败：网络连接错误，请检查网络连接");
    } else {
      alert(`检查更新失败: ${errorMsg}`);
    }
    updateAvailable.value = false;
  } finally {
    checkingUpdate.value = false;
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
  background-color: rgba(255, 255, 255, 0.1);
  color: white;
  padding: 0.5rem 1rem;
  font-size: 0.875rem;
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.2s;
}

.update-btn:hover {
  background-color: rgba(255, 255, 255, 0.15);
}

.update-btn.update-available {
  background-color: #646cff;
  border-color: #646cff;
}

.update-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
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

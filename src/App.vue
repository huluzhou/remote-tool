<template>
  <div class="app-container">
    <header class="app-header">
      <h1>Remote Tool</h1>
      <div class="header-actions">
        <!-- 更新按钮已隐藏，但保留代码逻辑用于后台自动检查更新 -->
        <button 
          @click="checkUpdate" 
          class="update-btn" 
          :class="{ 'update-available': updateAvailable }"
          :disabled="checkingUpdate"
          style="display: none;"
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

// 解析错误信息，提供更友好的提示
const parseUpdateError = (error: unknown): string => {
  const errorMsg = error instanceof Error ? error.message : String(error);
  const errorStr = errorMsg.toLowerCase();
  
  // 记录完整的错误信息到控制台
  console.error("更新检查错误详情:", {
    message: errorMsg,
    error: error,
    type: error instanceof Error ? error.constructor.name : typeof error,
    stack: error instanceof Error ? error.stack : undefined
  });
  
  // 网络请求发送失败（常见于 URL 错误或网络问题）
  if (
    errorStr.includes("error sending request") ||
    errorStr.includes("failed to send request") ||
    errorStr.includes("request failed") ||
    errorStr.includes("send request")
  ) {
    // 尝试提取 URL 信息
    const urlMatch = errorMsg.match(/https?:\/\/[^\s)]+/);
    if (urlMatch) {
      const url = urlMatch[0];
      // 检查是否是 GitHub URL
      if (url.includes("github.com")) {
        return `无法连接到更新服务器 ${url}。\n\n可能的原因：\n1. 网络连接问题或防火墙阻止\n2. GitHub 访问受限\n3. 该版本尚未发布更新（latest.json 文件不存在）\n\n提示：可以在浏览器中访问该 URL 确认文件是否存在。`;
      }
      return `网络请求失败: 无法连接到 ${url}。请检查网络连接或确认更新服务器是否可访问。`;
    }
    return "网络请求失败，请检查网络连接或防火墙设置。如果问题持续，可能是更新服务器暂时不可用。";
  }
  
  // 网络相关错误
  if (
    errorStr.includes("network") ||
    errorStr.includes("connection") ||
    errorStr.includes("timeout") ||
    errorStr.includes("econnrefused") ||
    errorStr.includes("enotfound") ||
    errorStr.includes("failed to fetch") ||
    errorStr.includes("networkerror") ||
    errorStr.includes("network request failed") ||
    errorStr.includes("connection refused") ||
    errorStr.includes("connection reset")
  ) {
    return "网络连接错误，请检查网络连接或防火墙设置";
  }
  
  // 404 或资源不存在
  if (
    errorStr.includes("404") ||
    errorStr.includes("not found") ||
    errorStr.includes("notfound") ||
    errorStr.includes("no such file")
  ) {
    return "未找到更新服务器或当前版本信息，可能该版本尚未发布更新或 latest.json 文件不存在";
  }
  
  // 403 权限错误
  if (errorStr.includes("403") || errorStr.includes("forbidden")) {
    return "访问更新服务器被拒绝，请稍后重试";
  }
  
  // 500 服务器错误
  if (errorStr.includes("500") || errorStr.includes("internal server error")) {
    return "更新服务器错误，请稍后重试";
  }
  
  // SSL/TLS 证书错误
  if (
    errorStr.includes("certificate") ||
    errorStr.includes("ssl") ||
    errorStr.includes("tls") ||
    errorStr.includes("cert")
  ) {
    return "SSL 证书验证失败，请检查系统时间或网络设置";
  }
  
  // DNS 解析错误
  if (
    errorStr.includes("dns") ||
    errorStr.includes("name resolution") ||
    errorStr.includes("could not resolve")
  ) {
    return "DNS 解析失败，无法解析更新服务器地址，请检查网络设置";
  }
  
  // 其他错误，返回原始错误信息（显示更多字符以便调试）
  const maxLength = 200;
  if (errorMsg.length > maxLength) {
    return errorMsg.substring(0, maxLength) + "...";
  }
  return errorMsg;
};

// 检查更新（不自动安装，静默失败）
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
    // 启动时检查更新失败，静默处理，不打扰用户
    console.warn("启动时检查更新失败（已静默处理）:", error);
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
        const errorMsg = parseUpdateError(installError);
        alert(`下载/安装更新失败: ${errorMsg}`);
      }
    } else {
      // 没有更新时，显示提示
      updateAvailable.value = false;
      alert("已是最新版本");
    }
  } catch (error) {
    // 检查更新失败
    const errorMsg = parseUpdateError(error);
    console.error("检查更新失败:", error);
    alert(`检查更新失败：${errorMsg}`);
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

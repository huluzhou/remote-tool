<template>
  <div class="deploy-form">
    <h3>部署配置</h3>
    <div class="files-list">
      <div v-for="(file, index) in formData.files" :key="index" class="file-item">
        <div class="file-item-header">
          <span class="file-item-label">文件 {{ index + 1 }}</span>
          <button
            v-if="formData.files.length > 1"
            type="button"
            @click="removeFile(index)"
            class="remove-btn"
          >
            删除
          </button>
        </div>
        <div class="file-input-group">
          <label>本地路径（上传）</label>
          <div class="input-with-btn">
            <input
              v-model="file.localPath"
              type="text"
              placeholder="选择本地文件用于上传"
              @input="saveToLocalStorage"
            />
            <button type="button" @click="selectLocalFile(index)" class="browse-btn">
              浏览
            </button>
          </div>
        </div>
        <div class="file-input-group">
          <label>远程路径</label>
          <input
            v-model="file.remotePath"
            type="text"
            placeholder="远程服务器文件路径"
            @input="saveToLocalStorage"
          />
        </div>
        <div class="file-input-group">
          <label>下载路径（下载）</label>
          <div class="input-with-btn">
            <input
              v-model="file.downloadPath"
              type="text"
              placeholder="选择本地保存路径用于下载"
              @input="saveToLocalStorage"
            />
            <button type="button" @click="selectDownloadPath(index)" class="browse-btn">
              浏览
            </button>
          </div>
        </div>
      </div>
      <button type="button" @click="addFile" class="add-file-btn">
        + 添加文件
      </button>
    </div>
    <div class="form-group">
      <label>
        <input
          v-model="formData.useRoot"
          type="checkbox"
          @change="saveToLocalStorage"
        />
        使用root用户
      </label>
    </div>
    <div class="action-buttons">
      <button
        type="button"
        @click="handleUpload"
        class="action-btn upload-btn"
        :disabled="deploying"
      >
        {{ deploying ? "上传中..." : "上传选中文件" }}
      </button>
      <button
        type="button"
        @click="handleDownload"
        class="action-btn download-btn"
        :disabled="deploying"
      >
        {{ deploying ? "下载中..." : "下载选中文件" }}
      </button>
      <button
        type="button"
        @click="handleRestart"
        class="action-btn restart-btn"
        :disabled="deploying"
      >
        {{ deploying ? "重启中..." : "重启服务" }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted } from "vue";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useDeployStore, type DeployConfig, type DeployFile } from "../../stores/deploy";

const emit = defineEmits<{
  deploy: [config: DeployConfig];
}>();

const deployStore = useDeployStore();
const deploying = ref(false);

// 同步 store 的 deploying 状态
watch(() => deployStore.deploying, (newValue) => {
  deploying.value = newValue;
}, { immediate: true });

// 默认的3个文件项，预填远程路径
const getDefaultFiles = (): DeployFile[] => {
  return [
    {
      remotePath: "/opt/analysis/bin/ancol",
    },
    {
      remotePath: "/opt/analysis/config.toml",
    },
    {
      remotePath: "/opt/analysis/topo.json",
    },
  ];
};

// 从localStorage加载配置
const loadFromLocalStorage = (): { files: DeployFile[]; useRoot: boolean } => {
  try {
    const saved = localStorage.getItem("deploy-config");
    if (saved) {
      const parsed = JSON.parse(saved);
      return {
        files: parsed.files && parsed.files.length > 0 ? parsed.files : getDefaultFiles(),
        useRoot: parsed.useRoot !== undefined ? parsed.useRoot : true,
      };
    }
  } catch (error) {
    console.error("加载保存的配置失败:", error);
  }
  return {
    files: getDefaultFiles(),
    useRoot: true,
  };
};

// 保存到localStorage
const saveToLocalStorage = () => {
  try {
    localStorage.setItem("deploy-config", JSON.stringify({
      files: formData.value.files,
      useRoot: formData.value.useRoot,
    }));
  } catch (error) {
    console.error("保存配置失败:", error);
  }
};

const formData = ref<{ files: DeployFile[]; useRoot: boolean }>(loadFromLocalStorage());

// 组件挂载时加载配置
onMounted(() => {
  const saved = loadFromLocalStorage();
  formData.value = saved;
});

const addFile = () => {
  formData.value.files.push({
    localPath: "",
    remotePath: "",
    downloadPath: "",
  });
  saveToLocalStorage();
};

const removeFile = (index: number) => {
  if (formData.value.files.length > 1) {
    formData.value.files.splice(index, 1);
    saveToLocalStorage();
  }
};

const selectLocalFile = async (index: number) => {
  try {
    const file = await open({
      multiple: false,
    });
    if (file) {
      const filePath = typeof file === "string" ? file : (file as any)?.path || String(file);
      formData.value.files[index].localPath = filePath;
      saveToLocalStorage();
    }
  } catch (error) {
    console.error("选择文件失败:", error);
    alert(`选择文件失败: ${error instanceof Error ? error.message : String(error)}`);
  }
};

const selectDownloadPath = async (index: number) => {
  try {
    // 从远程路径获取文件名作为默认文件名
    const remotePath = formData.value.files[index].remotePath;
    const defaultFileName = remotePath ? remotePath.split('/').pop() || 'file' : 'file';
    
    const filePath = await save({
      defaultPath: defaultFileName,
    });
    if (filePath) {
      formData.value.files[index].downloadPath = filePath;
      saveToLocalStorage();
    }
  } catch (error) {
    console.error("选择下载路径失败:", error);
    alert(`选择下载路径失败: ${error instanceof Error ? error.message : String(error)}`);
  }
};

// 验证上传文件
const validateUploadFiles = (): boolean => {
  const uploadFiles = formData.value.files.filter(
    (f) => f.localPath && f.localPath.trim() && f.remotePath && f.remotePath.trim()
  );
  if (uploadFiles.length === 0) {
    alert("请至少选择一个文件进行上传（需要填写本地路径和远程路径）");
    return false;
  }
  return true;
};

// 验证下载文件
const validateDownloadFiles = (): boolean => {
  const downloadFiles = formData.value.files.filter(
    (f) => f.remotePath && f.remotePath.trim() && f.downloadPath && f.downloadPath.trim()
  );
  if (downloadFiles.length === 0) {
    alert("请至少选择一个文件进行下载（需要填写远程路径和下载路径）");
    return false;
  }
  return true;
};

// 处理上传
const handleUpload = () => {
  if (deploying.value) {
    return;
  }
  
  if (!validateUploadFiles()) {
    return;
  }
  
  // 只包含有本地路径和远程路径的文件
  const filesToUpload = formData.value.files
    .filter((f) => f.localPath && f.localPath.trim() && f.remotePath && f.remotePath.trim())
    .map((f) => ({
      localPath: f.localPath,
      remotePath: f.remotePath,
      downloadPath: undefined,
    }));
  
  const deployConfig: DeployConfig = {
    files: filesToUpload,
    useRoot: formData.value.useRoot,
    restartService: true, // 上传默认重启服务
  };
  
  emit("deploy", deployConfig);
};

// 处理下载
const handleDownload = () => {
  if (deploying.value) {
    return;
  }
  
  if (!validateDownloadFiles()) {
    return;
  }
  
  // 只包含有远程路径和下载路径的文件
  const filesToDownload = formData.value.files
    .filter((f) => f.remotePath && f.remotePath.trim() && f.downloadPath && f.downloadPath.trim())
    .map((f) => ({
      localPath: undefined,
      remotePath: f.remotePath,
      downloadPath: f.downloadPath,
    }));
  
  const deployConfig: DeployConfig = {
    files: filesToDownload,
    useRoot: formData.value.useRoot,
    restartService: false, // 下载默认不重启服务
  };
  
  emit("deploy", deployConfig);
};

// 处理重启服务
const handleRestart = () => {
  if (deploying.value) {
    return;
  }
  
  const deployConfig: DeployConfig = {
    files: [], // 不传输任何文件
    useRoot: formData.value.useRoot,
    restartService: true,
  };
  
  emit("deploy", deployConfig);
};
</script>

<style scoped>
.deploy-form {
  padding: 1.5rem;
  background-color: rgba(255, 255, 255, 0.05);
  border-radius: 8px;
}

.deploy-form h3 {
  margin-bottom: 1rem;
  font-size: 1.25rem;
}

.files-list {
  margin-bottom: 1.5rem;
}

.file-item {
  margin-bottom: 1.5rem;
  padding: 1rem;
  background-color: rgba(255, 255, 255, 0.03);
  border-radius: 6px;
  border: 2px solid rgba(255, 255, 255, 0.2);
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

.file-item-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.75rem;
}

.file-item-label {
  font-weight: 500;
  font-size: 0.875rem;
  color: rgba(255, 255, 255, 0.8);
}

.remove-btn {
  padding: 0.25rem 0.75rem;
  background-color: rgba(255, 0, 0, 0.2);
  color: #ff6b6b;
  border: 1px solid rgba(255, 0, 0, 0.3);
  border-radius: 4px;
  font-size: 0.75rem;
  cursor: pointer;
}

.remove-btn:hover {
  background-color: rgba(255, 0, 0, 0.3);
}

.file-input-group {
  margin-bottom: 0.75rem;
}

.file-input-group label {
  display: block;
  margin-bottom: 0.5rem;
  font-size: 0.875rem;
  color: rgba(255, 255, 255, 0.7);
}

.input-with-btn {
  display: flex;
  gap: 0.5rem;
}

.input-with-btn input {
  flex: 1;
  padding: 0.5rem;
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 4px;
  background-color: rgba(255, 255, 255, 0.05);
  color: inherit;
  font-size: 0.875rem;
}

.file-input-group input {
  width: 100%;
  padding: 0.5rem;
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 4px;
  background-color: rgba(255, 255, 255, 0.05);
  color: inherit;
  font-size: 0.875rem;
}

.browse-btn {
  padding: 0.5rem 1rem;
  background-color: rgba(255, 255, 255, 0.1);
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 4px;
  color: inherit;
  font-size: 0.875rem;
  cursor: pointer;
  white-space: nowrap;
}

.browse-btn:hover {
  background-color: rgba(255, 255, 255, 0.15);
}

.add-file-btn {
  width: 100%;
  padding: 0.75rem;
  background-color: rgba(100, 108, 255, 0.2);
  border: 1px dashed rgba(100, 108, 255, 0.5);
  border-radius: 4px;
  color: #646cff;
  font-size: 0.875rem;
  cursor: pointer;
  margin-top: 0.5rem;
}

.add-file-btn:hover {
  background-color: rgba(100, 108, 255, 0.3);
}

.form-group {
  margin-bottom: 1rem;
}

.form-group label {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.875rem;
  cursor: pointer;
}

.action-buttons {
  display: flex;
  gap: 0.75rem;
  margin-top: 1.5rem;
}

.action-btn {
  flex: 1;
  padding: 0.75rem;
  border: none;
  border-radius: 4px;
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  transition: opacity 0.2s;
}

.action-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.upload-btn {
  background-color: #646cff;
  color: white;
}

.upload-btn:hover:not(:disabled) {
  background-color: #535bf2;
}

.download-btn {
  background-color: #10b981;
  color: white;
}

.download-btn:hover:not(:disabled) {
  background-color: #059669;
}

.restart-btn {
  background-color: #f59e0b;
  color: white;
}

.restart-btn:hover:not(:disabled) {
  background-color: #d97706;
}

@media (max-width: 768px) {
  .action-buttons {
    flex-direction: column;
  }
}
</style>

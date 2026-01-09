<template>
  <div class="deploy-form">
    <h3>部署配置</h3>
    <form @submit.prevent="handleSubmit">
      <div class="form-group">
        <label>可执行文件路径:</label>
        <div class="file-input-group">
          <input
            v-model="formData.binaryPath"
            type="text"
            placeholder="选择可执行文件"
            required
          />
          <button type="button" @click="selectBinaryFile" class="browse-btn">
            浏览
          </button>
        </div>
      </div>
      <div class="form-group">
        <label>
          <input
            v-model="formData.uploadConfig"
            type="checkbox"
          />
          上传配置文件 (config.toml)
        </label>
        <div v-if="formData.uploadConfig" class="file-input-group">
          <input
            v-model="formData.configPath"
            type="text"
            placeholder="选择配置文件"
          />
          <button type="button" @click="selectConfigFile" class="browse-btn">
            浏览
          </button>
        </div>
      </div>
      <div class="form-group">
        <label>
          <input
            v-model="formData.uploadTopo"
            type="checkbox"
          />
          上传拓扑文件 (topo.json)
        </label>
        <div v-if="formData.uploadTopo" class="file-input-group">
          <input
            v-model="formData.topoPath"
            type="text"
            placeholder="选择拓扑文件"
          />
          <button type="button" @click="selectTopoFile" class="browse-btn">
            浏览
          </button>
        </div>
      </div>
      <div class="form-group">
        <label>运行用户:</label>
        <div class="radio-group">
          <label>
            <input
              v-model="formData.useRoot"
              type="radio"
              :value="false"
            />
            普通用户 (analysis)
          </label>
          <label>
            <input
              v-model="formData.useRoot"
              type="radio"
              :value="true"
            />
            Root用户
          </label>
        </div>
      </div>
      <div class="form-group">
        <label>
          <input
            v-model="formData.startService"
            type="checkbox"
          />
          部署后启动服务
        </label>
      </div>
      <button type="submit" class="submit-btn" :disabled="deploying">
        {{ deploying ? "部署中..." : "开始部署" }}
      </button>
    </form>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";

const emit = defineEmits<{
  deploy: [config: any];
}>();

const deploying = ref(false);

const formData = ref({
  binaryPath: "",
  configPath: "",
  topoPath: "",
  uploadConfig: true,
  uploadTopo: true,
  useRoot: true,
  startService: true,
});

const selectBinaryFile = async () => {
  const file = await open({
    filters: [{ name: "可执行文件", extensions: ["*"] }],
  });
  if (file) {
    formData.value.binaryPath = typeof file === "string" ? file : file.path;
  }
};

const selectConfigFile = async () => {
  const file = await open({
    filters: [{ name: "配置文件", extensions: ["toml"] }],
  });
  if (file) {
    formData.value.configPath = typeof file === "string" ? file : file.path;
  }
};

const selectTopoFile = async () => {
  const file = await open({
    filters: [{ name: "JSON文件", extensions: ["json"] }],
  });
  if (file) {
    formData.value.topoPath = typeof file === "string" ? file : file.path;
  }
};

const handleSubmit = () => {
  deploying.value = true;
  emit("deploy", { ...formData.value });
  setTimeout(() => {
    deploying.value = false;
  }, 100);
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

.form-group {
  margin-bottom: 1rem;
}

.form-group label {
  display: block;
  margin-bottom: 0.5rem;
  font-size: 0.875rem;
  font-weight: 500;
}

.file-input-group {
  display: flex;
  gap: 0.5rem;
  margin-top: 0.5rem;
}

.file-input-group input {
  flex: 1;
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

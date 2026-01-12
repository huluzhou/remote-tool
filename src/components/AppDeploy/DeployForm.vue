<template>
  <div class="deploy-form">
    <h3>部署配置</h3>
    <form @submit.prevent="handleSubmit">
      <div class="form-group">
        <label>
          <input
            v-model="formData.uploadBinary"
            type="checkbox"
          />
          上传可执行文件
        </label>
        <div v-if="formData.uploadBinary" class="file-input-group">
          <input
            v-model="formData.binaryPath"
            type="text"
            placeholder="选择可执行文件"
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
  uploadBinary: false,  // 默认不上传，允许独立选择
  uploadConfig: false,  // 默认不上传，允许独立选择
  uploadTopo: false,    // 默认不上传，允许独立选择
  useRoot: true,        // 默认使用root用户
  startService: true,
});

const selectBinaryFile = async () => {
  console.log("[DEBUG] 点击浏览按钮 - 选择可执行文件");
  console.log("[DEBUG] open 函数是否存在:", typeof open);
  try {
    console.log("[DEBUG] 调用 open() 函数...");
    // 不设置 filters 或使用空数组，以显示所有文件（包括无扩展名的可执行文件）
    const file = await open({
      multiple: false,
      // 不设置 filters 会显示所有文件，包括无扩展名的文件
      // 或者使用空数组来显示所有文件
    });
    console.log("[DEBUG] open() 返回结果:", file, "类型:", typeof file);
    if (file) {
      // Tauri 2.0 的 open 函数返回的是字符串路径或 null
      const filePath = typeof file === "string" ? file : (file as any)?.path || String(file);
      console.log("[DEBUG] 设置文件路径:", filePath);
      formData.value.binaryPath = filePath;
    } else {
      console.log("[DEBUG] 用户取消了文件选择");
    }
  } catch (error) {
    console.error("[ERROR] 选择文件失败:", error);
    console.error("[ERROR] 错误详情:", error instanceof Error ? error.stack : error);
    alert(`选择文件失败: ${error instanceof Error ? error.message : String(error)}\n请查看控制台获取详细信息`);
  }
};

const selectConfigFile = async () => {
  console.log("[DEBUG] 点击浏览按钮 - 选择配置文件");
  console.log("[DEBUG] open 函数是否存在:", typeof open);
  try {
    console.log("[DEBUG] 调用 open() 函数...");
    const file = await open({
      multiple: false,
      filters: [{ name: "配置文件", extensions: ["toml"] }],
    });
    console.log("[DEBUG] open() 返回结果:", file, "类型:", typeof file);
    if (file) {
      const filePath = typeof file === "string" ? file : (file as any)?.path || String(file);
      console.log("[DEBUG] 设置文件路径:", filePath);
      formData.value.configPath = filePath;
    } else {
      console.log("[DEBUG] 用户取消了文件选择");
    }
  } catch (error) {
    console.error("[ERROR] 选择文件失败:", error);
    console.error("[ERROR] 错误详情:", error instanceof Error ? error.stack : error);
    alert(`选择文件失败: ${error instanceof Error ? error.message : String(error)}\n请查看控制台获取详细信息`);
  }
};

const selectTopoFile = async () => {
  console.log("[DEBUG] 点击浏览按钮 - 选择拓扑文件");
  console.log("[DEBUG] open 函数是否存在:", typeof open);
  try {
    console.log("[DEBUG] 调用 open() 函数...");
    const file = await open({
      multiple: false,
      filters: [{ name: "JSON文件", extensions: ["json"] }],
    });
    console.log("[DEBUG] open() 返回结果:", file, "类型:", typeof file);
    if (file) {
      const filePath = typeof file === "string" ? file : (file as any)?.path || String(file);
      console.log("[DEBUG] 设置文件路径:", filePath);
      formData.value.topoPath = filePath;
    } else {
      console.log("[DEBUG] 用户取消了文件选择");
    }
  } catch (error) {
    console.error("[ERROR] 选择文件失败:", error);
    console.error("[ERROR] 错误详情:", error instanceof Error ? error.stack : error);
    alert(`选择文件失败: ${error instanceof Error ? error.message : String(error)}\n请查看控制台获取详细信息`);
  }
};

const handleSubmit = () => {
  // 验证至少选择了一种文件上传
  if (!formData.value.uploadBinary && !formData.value.uploadConfig && !formData.value.uploadTopo) {
    alert("请至少选择一种文件进行上传");
    return;
  }
  
  // 验证如果选择了上传，必须提供文件路径
  if (formData.value.uploadBinary && !formData.value.binaryPath) {
    alert("请选择可执行文件");
    return;
  }
  if (formData.value.uploadConfig && !formData.value.configPath) {
    alert("请选择配置文件");
    return;
  }
  if (formData.value.uploadTopo && !formData.value.topoPath) {
    alert("请选择拓扑文件");
    return;
  }
  
  deploying.value = true;
  // 确保所有必需字段都被发送，即使值为 false
  const deployConfig = {
    binaryPath: formData.value.binaryPath || undefined,
    configPath: formData.value.configPath || undefined,
    topoPath: formData.value.topoPath || undefined,
    uploadBinary: formData.value.uploadBinary || false,
    uploadConfig: formData.value.uploadConfig !== undefined ? formData.value.uploadConfig : false,
    uploadTopo: formData.value.uploadTopo !== undefined ? formData.value.uploadTopo : false,
    useRoot: formData.value.useRoot !== undefined ? formData.value.useRoot : true,
    startService: formData.value.startService !== undefined ? formData.value.startService : true,
  };
  emit("deploy", deployConfig);
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

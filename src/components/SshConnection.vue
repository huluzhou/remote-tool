<template>
  <div class="ssh-connection">
    <div class="connection-form">
      <div class="form-group">
        <label>SSH 连接指令:</label>
        <input
          v-model="sshCommand"
          type="text"
          placeholder="例如: ssh user@192.168.1.100 -p 22"
        />
      </div>
      <div class="form-group">
        <label>密码:</label>
        <input
          v-model="password"
          type="password"
          placeholder="请输入SSH登录密码（必填）"
        />
      </div>
      <div class="form-actions">
        <button
          @click="handleConnect"
          :disabled="connecting || connected"
          class="connect-btn"
        >
          {{ connecting ? "连接中..." : connected ? "已连接" : "连接" }}
        </button>
        <button
          v-if="connected"
          @click="handleDisconnect"
          class="disconnect-btn"
        >
          断开
        </button>
        <span :class="['status', { connected, error: error }]">
          {{ statusText }}
        </span>
      </div>
      <div v-if="error" class="error-message">{{ error }}</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useSshStore } from "../stores/ssh";

const sshStore = useSshStore();

const sshCommand = ref("");
const password = ref("");
const connecting = ref(false);
const error = ref("");

const connected = computed(() => sshStore.isConnected);

const statusText = computed(() => {
  if (connecting.value) return "连接中...";
  if (connected.value) return "已连接";
  if (error.value) return "连接失败";
  return "未连接";
});

const parseSshCommand = (command: string): {
  username: string;
  host: string;
  port: number;
} | null => {
  const trimmed = command.trim();
  if (!trimmed) return null;

  // 移除 ssh 前缀
  const withoutSsh = trimmed.replace(/^ssh\s+/i, "");

  // 提取端口
  const portMatch = withoutSsh.match(/-p\s+(\d+)/i);
  const port = portMatch ? parseInt(portMatch[1]) : 22;
  const withoutPort = withoutSsh.replace(/-p\s+\d+/i, "").trim();

  // 提取用户名和主机
  const match = withoutPort.match(/([^@]+)@([^\s]+)/);
  if (!match) return null;

  return {
    username: match[1].trim(),
    host: match[2].trim(),
    port,
  };
};

const handleConnect = async () => {
  const parsed = parseSshCommand(sshCommand.value);
  if (!parsed) {
    error.value = "请输入有效的SSH连接指令";
    return;
  }

  if (!password.value) {
    error.value = "请输入密码";
    return;
  }

  connecting.value = true;
  error.value = "";

  const config = {
    host: parsed.host,
    port: parsed.port,
    username: parsed.username,
    password: password.value,
  };

  const result = await sshStore.connect(config);

  connecting.value = false;

  if (result.success) {
    // 连接成功，自动保存配置
    saveSshConfig(sshCommand.value, password.value);
  } else {
    error.value = result.error || "连接失败";
  }
};

const handleDisconnect = async () => {
  await sshStore.disconnect();
  error.value = "";
};


// 保存SSH配置到localStorage
const saveSshConfig = (command: string, pwd: string) => {
  try {
    const config = {
      sshCommand: command,
      password: pwd, // 注意：密码以明文保存，实际应用中应该加密
      savedAt: new Date().toISOString(),
    };
    localStorage.setItem("ssh_config", JSON.stringify(config));
  } catch (e) {
    console.error("保存SSH配置失败:", e);
  }
};

// 从localStorage加载SSH配置
const loadSshConfig = () => {
  try {
    const saved = localStorage.getItem("ssh_config");
    if (saved) {
      const config = JSON.parse(saved);
      sshCommand.value = config.sshCommand || "";
      password.value = config.password || "";
    }
  } catch (e) {
    console.error("加载SSH配置失败:", e);
  }
};

onMounted(() => {
  // 优先从localStorage加载保存的配置
  loadSshConfig();
  
  // 如果localStorage中没有，则从store加载当前连接
  if (!sshCommand.value && sshStore.currentConnection) {
    const conn = sshStore.currentConnection;
    sshCommand.value = `ssh ${conn.config.username}@${conn.config.host} -p ${conn.config.port}`;
    password.value = conn.config.password || "";
  }
});
</script>

<style scoped>
.ssh-connection {
  padding: 1rem;
  background-color: rgba(255, 255, 255, 0.05);
  border-radius: 8px;
  margin-bottom: 1.5rem;
}

.connection-form {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.form-group label {
  font-size: 0.875rem;
  font-weight: 500;
}

.form-group input {
  padding: 0.5rem;
  border: 1.5px solid #d0d0d0;
  border-radius: 4px;
  background-color: #ffffff;
  color: #213547;
  font-size: 0.875rem;
  transition: all 0.2s;
  width: 100%;
  box-sizing: border-box;
}

/* 深色主题下的边框 */
@media (prefers-color-scheme: dark) {
  .form-group input {
    border-color: rgba(255, 255, 255, 0.4);
    background-color: rgba(255, 255, 255, 0.05);
    color: rgba(255, 255, 255, 0.87);
  }
}

.form-group input:hover {
  border-color: #999999;
}

@media (prefers-color-scheme: dark) {
  .form-group input:hover {
    border-color: rgba(255, 255, 255, 0.6);
    background-color: rgba(255, 255, 255, 0.08);
  }
}

.form-group input:focus {
  outline: none;
  border-color: #646cff;
  background-color: #ffffff;
  box-shadow: 0 0 0 2px rgba(100, 108, 255, 0.2);
}

@media (prefers-color-scheme: dark) {
  .form-group input:focus {
    background-color: rgba(255, 255, 255, 0.1);
  }
}

.form-group input::placeholder {
  color: #999999;
  opacity: 1;
}

/* 深色主题下的 placeholder */
@media (prefers-color-scheme: dark) {
  .form-group input::placeholder {
    color: rgba(255, 255, 255, 0.5);
  }
}

.form-group input:placeholder-shown {
  color: #999999;
}

@media (prefers-color-scheme: dark) {
  .form-group input:placeholder-shown {
    color: rgba(255, 255, 255, 0.7);
  }
}

.form-actions {
  display: flex;
  align-items: center;
  gap: 1rem;
}

.connect-btn,
.disconnect-btn {
  padding: 0.5rem 1rem;
  font-size: 0.875rem;
}

.connect-btn {
  background-color: #646cff;
  color: white;
}

.connect-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.disconnect-btn {
  background-color: #dc3545;
  color: white;
}

.status {
  font-size: 0.875rem;
  padding: 0.25rem 0.5rem;
  border-radius: 4px;
}

.status.connected {
  color: #4caf50;
}

.status.error {
  color: #f44336;
}

.error-message {
  color: #f44336;
  font-size: 0.875rem;
  padding: 0.5rem;
  background-color: rgba(244, 67, 54, 0.1);
  border-radius: 4px;
}
</style>

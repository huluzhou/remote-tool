<template>
  <div class="deploy-view">
    <SshConnection />
    <div v-if="sshStore.isConnected" class="deploy-container">
      <div class="deploy-sections">
        <DeployForm @deploy="handleDeploy" />
        <StatusCheck @check="handleCheckStatus" />
      </div>
      <DeployLog 
        :logs="deployStore.logs" 
        :error="deployStore.error"
        @clear="deployStore.clearLogs"
      />
    </div>
    <div v-else class="not-connected">
      <p>请先连接SSH服务器</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted } from "vue";
import { useSshStore } from "../stores/ssh";
import { useDeployStore } from "../stores/deploy";
import SshConnection from "../components/SshConnection.vue";
import DeployForm from "../components/AppDeploy/DeployForm.vue";
import DeployLog from "../components/AppDeploy/DeployLog.vue";
import StatusCheck from "../components/AppDeploy/StatusCheck.vue";

const sshStore = useSshStore();
const deployStore = useDeployStore();

const handleDeploy = async (config: any) => {
  await deployStore.deploy(config);
};

const handleCheckStatus = async () => {
  await deployStore.checkStatus();
};

onMounted(() => {
  if (sshStore.isConnected) {
    handleCheckStatus();
  }
});
</script>

<style scoped>
.deploy-view {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.deploy-container {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.deploy-sections {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1.5rem;
}

.not-connected {
  padding: 2rem;
  text-align: center;
  color: rgba(255, 255, 255, 0.5);
}

@media (max-width: 768px) {
  .deploy-sections {
    grid-template-columns: 1fr;
  }
}
</style>

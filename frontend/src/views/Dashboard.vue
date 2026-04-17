<template>
  <div class="dashboard">
    <div class="stats-grid">
      <stat-card
        title="服务器总数"
        :value="serversStore.servers.length"
        color="#50C878"
      />
      <stat-card
        title="运行中"
        :value="serversStore.runningCount"
        color="#64B5F6"
      />
      <stat-card
        title="在线玩家"
        :value="serversStore.totalPlayers"
        color="#FFB347"
      />
      <stat-card
        title="系统负载"
        :value="systemLoad"
        color="#FF6B6B"
      />
    </div>

    <div class="content-section">
      <h2>服务器状态</h2>
      <n-spin :show="loading">
        <div class="servers-grid">
          <server-card
            v-for="server in serversStore.servers"
            :key="server.id"
            :server="server"
            @start="() => handleStartServer(server.id)"
            @stop="() => handleStopServer(server.id)"
            @click="() => goToServerDetail(server.id)"
          />
        </div>
      </n-spin>
    </div>

    <div class="content-section">
      <h2>最近操作日志</h2>
      <n-spin :show="logsLoading">
        <n-data-table :columns="logColumns" :data="logs" :single-line="false" :bordered="false" />
      </n-spin>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { NSpin, NDataTable } from 'naive-ui'
import { useServersStore } from '../stores/servers'
import { systemApi } from '../api/system'
import { useNotification } from '../composables/useNotification'
import StatCard from '../components/common/StatCard.vue'
import ServerCard from '../components/server/ServerCard.vue'

const router = useRouter()
const serversStore = useServersStore()
const notification = useNotification()

const loading = ref(false)
const logsLoading = ref(false)
const systemLoad = ref('--')
const logs = ref<any[]>([])

const logColumns = [
  { title: '操作', key: 'action', width: 100 },
  { title: '目标', key: 'target', width: 150 },
  { title: '操作者', key: 'user_id', width: 120 },
  { title: '时间', key: 'created_at', width: 180 }
]

async function loadData() {
  loading.value = true
  try {
    await serversStore.fetchServers()
  } catch (error) {
    notification.error('加载服务器失败', '')
  } finally {
    loading.value = false
  }

  logsLoading.value = true
  try {
    const response = await systemApi.getOperationLogs(10)
    logs.value = response.data
  } catch (error) {
    notification.error('加载日志失败', '')
  } finally {
    logsLoading.value = false
  }

  try {
    const sysInfo = await systemApi.getSystemInfo()
    if (sysInfo.data.memory_usage !== undefined) {
      systemLoad.value = `${Math.round(sysInfo.data.memory_usage)}%`
    } else if (sysInfo.data.memory_total) {
      const memoryPercent = Math.round((sysInfo.data.memory_used / sysInfo.data.memory_total) * 100)
      systemLoad.value = `${memoryPercent}%`
    }
  } catch (error) {
    systemLoad.value = 'N/A'
  }
}

async function handleStartServer(serverId: string) {
  try {
    await serversStore.startServer(serverId)
    notification.success('服务器已启动', '')
  } catch (error: any) {
    notification.error('启动失败', error?.response?.data?.message || '')
  }
}

async function handleStopServer(serverId: string) {
  try {
    await serversStore.stopServer(serverId)
    notification.success('服务器已停止', '')
  } catch (error: any) {
    notification.error('停止失败', error?.response?.data?.message || '')
  }
}

function goToServerDetail(serverId: string) {
  router.push(`/servers/${serverId}`)
}

onMounted(() => {
  loadData()
})
</script>

<style scoped>
.dashboard {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 16px;
}

.content-section {
  background-color: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 20px;
  transition: background-color 0.3s, border-color 0.3s;
}

.content-section h2 {
  margin: 0 0 16px 0;
  color: var(--text-primary);
  font-size: 16px;
}

.servers-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
  gap: 16px;
}

:deep(.n-table) {
  --n-td-padding: 8px 12px;
  --n-th-padding: 12px;
}
</style>

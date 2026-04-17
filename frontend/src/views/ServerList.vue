<template>
  <div class="server-list">
    <div class="header">
      <h1>服务器管理</h1>
      <n-button type="primary" @click="showCreateModal = true">
        + 新建服务器
      </n-button>
    </div>

    <n-spin :show="loading">
      <n-data-table
        :columns="columns"
        :data="serversStore.servers"
        :single-line="false"
        striped
        :bordered="false"
      />
    </n-spin>

    <create-server-modal v-model:show="showCreateModal" @created="handleServerCreated" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, h } from 'vue'
import { useRouter } from 'vue-router'
import { NButton, NSpin, NDataTable, NSpace } from 'naive-ui'
import { useServersStore } from '../stores/servers'
import { useNotification } from '../composables/useNotification'
import CreateServerModal from '../components/server/CreateServerModal.vue'
import ServerStatusBadge from '../components/server/ServerStatusBadge.vue'

const router = useRouter()
const serversStore = useServersStore()
const notification = useNotification()

const showCreateModal = ref(false)
const loading = ref(false)

const columns = computed(() => [
  {
    title: '服务器名称',
    key: 'name',
    width: 150
  },
  {
    title: '状态',
    key: 'status',
    width: 100,
    render: (row: any) => h(ServerStatusBadge, { status: row.status })
  },
  {
    title: 'TShock 版本',
    key: 'tshock_version',
    width: 120
  },
  {
    title: '端口',
    key: 'port',
    width: 80
  },
  {
    title: '玩家数',
    key: 'player_count',
    width: 80
  },
  {
    title: '最大玩家',
    key: 'max_players',
    width: 100
  },
  {
    title: '操作',
    key: 'actions',
    width: 200,
    align: 'center' as const,
    render: (row: any) => h(
      NSpace,
      { size: 'small' },
      {
        default: () => [
          h(
            NButton,
            {
              text: true,
              type: row.status === 'running' ? 'error' : 'primary',
              size: 'small',
              onClick: () => row.status === 'running' ? handleStop(row.id) : handleStart(row.id)
            },
            { default: () => row.status === 'running' ? '停止' : '启动' }
          ),
          h(
            NButton,
            {
              text: true,
              type: 'info',
              size: 'small',
              onClick: () => router.push(`/servers/${row.id}`)
            },
            { default: () => '详情' }
          ),
          h(
            NButton,
            {
              text: true,
              type: 'error',
              size: 'small',
              onClick: () => handleDelete(row.id)
            },
            { default: () => '删除' }
          )
        ]
      }
    )
  }
])

async function loadServers() {
  loading.value = true
  try {
    await serversStore.fetchServers()
  } catch (error) {
    notification.error('加载服务器失败', '')
  } finally {
    loading.value = false
  }
}

async function handleStart(serverId: string) {
  try {
    await serversStore.startServer(serverId)
    notification.success('服务器已启动', '')
  } catch (error: any) {
    notification.error('启动失败', error?.response?.data?.message || '')
  }
}

async function handleStop(serverId: string) {
  try {
    await serversStore.stopServer(serverId)
    notification.success('服务器已停止', '')
  } catch (error: any) {
    notification.error('停止失败', error?.response?.data?.message || '')
  }
}

async function handleDelete(serverId: string) {
  try {
    await serversStore.deleteServer(serverId)
    notification.success('服务器已删除', '')
  } catch (error: any) {
    notification.error('删除失败', error?.response?.data?.message || '')
  }
}

function handleServerCreated() {
  showCreateModal.value = false
  loadServers()
}

onMounted(() => {
  loadServers()
})

</script>

<style scoped>
.server-list {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.header h1 {
  margin: 0;
  color: var(--text-primary);
}

.expand-content {
  padding: 12px;
  background-color: var(--bg-body);
  border-radius: 8px;
}
</style>

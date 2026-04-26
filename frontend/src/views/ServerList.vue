<template>
  <div class="server-list">
    <div class="header">
      <h1>服务器管理</h1>
      <div class="header-actions">
        <n-button @click="loadServers">
          刷新状态
        </n-button>
        <n-button v-if="authStore.isOperator" type="primary" @click="showCreateModal = true">
          + 新建服务器
        </n-button>
      </div>
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
import { NButton, NSpin, NDataTable, NSpace, useDialog } from 'naive-ui'
import { useAuthStore } from '../stores/auth'
import { useServersStore } from '../stores/servers'
import { useNotification } from '../composables/useNotification'
import CreateServerModal from '../components/server/CreateServerModal.vue'
import ServerStatusBadge from '../components/server/ServerStatusBadge.vue'

const router = useRouter()
const authStore = useAuthStore()
const serversStore = useServersStore()
const notification = useNotification()
const dialog = useDialog()

const showCreateModal = ref(false)
const loading = ref(false)

function isServerActive(status: string) {
  return status !== 'stopped'
}

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
    width: 260,
    align: 'center' as const,
    render: (row: any) => h(
      NSpace,
      { size: 'small' },
      {
        default: () => [
          ...(authStore.isOperator ? [
            h(
              NButton,
              {
                text: true,
                type: isServerActive(row.status) ? 'error' : 'primary',
                size: 'small',
                onClick: () => isServerActive(row.status) ? handleStop(row.id) : handleStart(row.id)
              },
              { default: () => isServerActive(row.status) ? '停止' : '启动' }
            )
          ] : []),
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
          ...(authStore.isOperator ? [
            h(
              NButton,
              {
                text: true,
                type: 'warning',
                size: 'small',
                onClick: () => handleKill(row.id)
              },
              { default: () => '强制结束' }
            )
          ] : []),
          ...(authStore.isAdmin ? [
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
          ] : [])
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

function getServerName(serverId: string): string {
  const s = serversStore.servers.find(s => s.id === serverId)
  return s?.name || serverId
}

function handleStart(serverId: string) {
  dialog.warning({
    title: '确认启动',
    content: `确定要启动服务器「${getServerName(serverId)}」吗？`,
    positiveText: '启动',
    negativeText: '取消',
    onPositiveClick: async () => {
      try {
        const result = await serversStore.startServer(serverId)
        notification.success('启动请求已发送', result?.message || '服务器正在启动中...')
        await loadServers()
      } catch (error: any) {
        notification.error('启动失败', error?.response?.data?.error || '请检查服务器配置和日志')
      }
    }
  })
}

function handleStop(serverId: string) {
  dialog.warning({
    title: '确认停止',
    content: `确定要停止服务器「${getServerName(serverId)}」吗？正在游戏中的玩家将被断开连接。`,
    positiveText: '停止',
    negativeText: '取消',
    onPositiveClick: async () => {
      try {
        await serversStore.stopServer(serverId)
        notification.success('服务器已停止', '服务器已安全关闭')
        await loadServers()
      } catch (error: any) {
        notification.error('停止失败', error?.response?.data?.error || '')
      }
    }
  })
}

function handleKill(serverId: string) {
  dialog.error({
    title: '确认强制结束',
    content: `确定要强制结束服务器「${getServerName(serverId)}」吗？这可能导致未保存的数据丢失！建议先尝试正常停止。`,
    positiveText: '强制结束',
    negativeText: '取消',
    onPositiveClick: async () => {
      try {
        const result = await serversStore.killServer(serverId)
        notification.success('强制结束信号已发送', result?.message || '进程已被终止')
        await loadServers()
      } catch (error: any) {
        notification.error('强制结束失败', error?.response?.data?.error || '')
      }
    }
  })
}

function handleDelete(serverId: string) {
  const serverName = getServerName(serverId)

  const runDelete = async (backupMode: 'keep' | 'delete') => {
    try {
      const result = await serversStore.deleteServer(serverId, backupMode)
      notification.success(
        backupMode === 'delete' ? '服务器和相关备份已删除' : '服务器已删除，备份已保留',
        result?.deleted_backup_count ? `已删除 ${result.deleted_backup_count} 个备份文件` : ''
      )
      await loadServers()
    } catch (error: any) {
      notification.error('删除失败', error?.response?.data?.message || error?.response?.data?.error || '')
    }
  }

  dialog.warning({
    title: '删除服务器并保留备份',
    content: `确定要删除服务器「${serverName}」吗？服务器配置和运行数据会被移除，但现有备份会保留，后续仍可在存档管理中单独删除。`,
    positiveText: '删除服务器，保留备份',
    negativeText: '取消',
    onPositiveClick: async () => {
      await runDelete('keep')
    },
    onNegativeClick: () => {
      dialog.error({
        title: '删除服务器和相关备份',
        content: `要把服务器「${serverName}」的相关备份也一起删除吗？这会同时删除该服务器生成的备份记录和磁盘文件，且不可恢复。`,
        positiveText: '删除服务器和备份',
        negativeText: '返回',
        onPositiveClick: async () => {
          await runDelete('delete')
        }
      })
    }
  })
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

.header-actions {
  display: flex;
  gap: 12px;
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

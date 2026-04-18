<template>
  <div class="user-manager">
    <div class="header">
      <h1>用户管理</h1>
      <n-button type="primary" @click="showCreateModal = true">
        + 新建用户
      </n-button>
    </div>

    <n-spin :show="loading">
      <n-data-table
        :columns="columns"
        :data="users"
        :single-line="false"
        striped
        :bordered="false"
      />
    </n-spin>

    <create-user-modal v-model:show="showCreateModal" @created="loadUsers" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, h } from 'vue'
import { NButton, NSpin, NDataTable, NSpace, NSelect } from 'naive-ui'
import { systemApi } from '../api/system'
import { useNotification } from '../composables/useNotification'
import CreateUserModal from '../components/user/CreateUserModal.vue'

const notification = useNotification()

const showCreateModal = ref(false)
const loading = ref(false)
const users = ref<any[]>([])
const roleOptions = [
  { label: '管理员', value: 'admin' },
  { label: '操作员', value: 'operator' },
  { label: '观察者', value: 'viewer' }
]

const columns = computed(() => [
  {
    title: '用户名',
    key: 'username',
    width: 150
  },
  {
    title: '角色',
    key: 'role',
    width: 160,
    render: (row: any) => h(NSelect, {
      value: row.role,
      size: 'small',
      style: 'width: 140px',
      options: roleOptions,
      onUpdateValue: (value: string) => handleUpdateRole(row, value)
    })
  },
  {
    title: '创建时间',
    key: 'created_at',
    width: 180
  },
  {
    title: '操作',
    key: 'actions',
    width: 150,
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
              type: 'error',
              size: 'small',
              onClick: () => handleDeleteUser(row.id)
            },
            { default: () => '删除' }
          )
        ]
      }
    )
  }
])

async function loadUsers() {
  loading.value = true
  try {
    const response = await systemApi.getUsers()
    users.value = response.data
  } catch (error) {
    notification.error('加载用户列表失败', '')
  } finally {
    loading.value = false
  }
}

async function handleDeleteUser(userId: string) {
  try {
    await systemApi.deleteUser(userId)
    notification.success('用户已删除', '')
    loadUsers()
  } catch (error: any) {
    notification.error('删除失败', error?.response?.data?.message || '')
  }
}

async function handleUpdateRole(user: any, role: string) {
  if (user.role === role) {
    return
  }

  try {
    await systemApi.updateUser(user.id, { role: role as 'admin' | 'operator' | 'viewer' })
    notification.success('角色已更新', `${user.username} 已设为 ${role}`)
    await loadUsers()
  } catch (error: any) {
    notification.error('更新角色失败', error?.response?.data?.error || '')
    await loadUsers()
  }
}

onMounted(() => {
  loadUsers()
})
</script>

<style scoped>
.user-manager {
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
</style>

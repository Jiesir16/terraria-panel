<template>
  <div class="tshock-manager">
    <!-- Summary -->
    <div class="section-header">
      <h3>TShock 用户与权限管理</h3>
      <n-button text type="primary" @click="loadAll" :loading="overviewLoading">刷新</n-button>
    </div>

    <n-spin :show="overviewLoading">
      <div class="security-summary" v-if="overview">
        <n-alert :type="overview.ssc_enabled ? 'success' : 'warning'" :show-icon="false">
          <div class="summary-line">
            <strong>SSC 状态：</strong>
            <span>{{ overview.ssc_enabled ? '已启用' : '未启用' }}</span>
            <n-tag size="small" :type="overview.ssc_enabled ? 'success' : 'warning'">{{ overview.ssc_source }}</n-tag>
          </div>
          <div class="summary-line">
            <strong>默认注册组：</strong>
            <span>{{ overview.default_registration_group || '未配置' }}</span>
          </div>
          <div class="summary-line">
            <strong>默认游客组：</strong>
            <span>{{ overview.default_guest_group || '未配置' }}</span>
          </div>
          <div class="summary-line" v-if="!overview.database_exists">
            <strong>数据库：</strong>
            <span>TShock 尚未生成 tshock.sqlite，先让服务器完整启动一次。</span>
          </div>
        </n-alert>
      </div>

      <!-- Sub-tabs for users / groups / SSC -->
      <n-tabs v-if="overview?.database_exists" type="segment" style="margin-top: 16px;">
        <!-- ─── Users Tab ─── -->
        <n-tab-pane name="users" tab="用户管理">
          <div class="sub-section">
            <div v-if="overview.users.length === 0" class="empty-note">暂无已注册 TShock 用户</div>
            <n-data-table
              v-else
              :columns="userColumns"
              :data="overview.users"
              :row-key="(row: TShockUserAccount) => row.username"
              :pagination="{ pageSize: 15 }"
              size="small"
              striped
            />
          </div>
        </n-tab-pane>

        <!-- ─── Groups Tab ─── -->
        <n-tab-pane name="groups" tab="组管理">
          <div class="sub-section">
            <div class="sub-header">
              <span></span>
              <n-button v-if="authStore.isAdmin" size="small" type="primary" @click="showCreateGroup = true">
                + 新建组
              </n-button>
            </div>
            <div v-if="overview.groups.length === 0" class="empty-note">暂无可读的 TShock 组信息</div>
            <n-data-table
              v-else
              :columns="groupColumns"
              :data="overview.groups"
              :row-key="(row: TShockGroupSummary) => row.name"
              :pagination="{ pageSize: 15 }"
              size="small"
              striped
            />
          </div>
        </n-tab-pane>

        <!-- ─── SSC Characters Tab ─── -->
        <n-tab-pane name="ssc" tab="SSC 角色">
          <div class="sub-section">
            <div class="sub-header">
              <span class="muted">
                {{ overview.ssc_enabled ? 'SSC 已启用，角色数据存储在服务端' : 'SSC 未启用' }}
              </span>
              <n-button v-if="authStore.isOperator" size="small" type="warning" @click="handleBackupSsc" :loading="sscBackupLoading">
                备份所有角色
              </n-button>
            </div>
            <n-spin :show="sscLoading">
              <div v-if="sscCharacters.length === 0" class="empty-note">暂无 SSC 角色数据</div>
              <n-data-table
                v-else
                :columns="sscColumns"
                :data="sscCharacters"
                :row-key="(row: TShockSscCharacterSummary) => row.account"
                :pagination="{ pageSize: 15 }"
                size="small"
                striped
              />
            </n-spin>
          </div>
        </n-tab-pane>
      </n-tabs>
    </n-spin>

    <!-- ─── Modals ─── -->

    <!-- Change User Group -->
    <n-modal v-model:show="showChangeGroup" preset="dialog" title="修改用户组">
      <div style="padding: 8px 0;">
        <p>将用户 <strong>{{ editingUser }}</strong> 移动到：</p>
        <n-select
          v-model:value="selectedGroup"
          :options="groupOptions"
          placeholder="选择目标组"
          filterable
        />
      </div>
      <template #action>
        <n-button @click="showChangeGroup = false">取消</n-button>
        <n-button type="primary" :loading="changingGroup" @click="confirmChangeGroup">确认</n-button>
      </template>
    </n-modal>

    <!-- Create Group -->
    <n-modal v-model:show="showCreateGroup" preset="dialog" title="新建 TShock 组">
      <div style="padding: 8px 0;">
        <n-form-item label="组名称" :show-feedback="false" style="margin-bottom: 12px;">
          <n-input v-model:value="newGroupName" placeholder="输入组名" />
        </n-form-item>
        <n-form-item label="父组 (可选)" :show-feedback="false">
          <n-select
            v-model:value="newGroupParent"
            :options="groupOptions"
            placeholder="选择父组"
            clearable
            filterable
          />
        </n-form-item>
      </div>
      <template #action>
        <n-button @click="showCreateGroup = false">取消</n-button>
        <n-button type="primary" :loading="creatingGroup" @click="confirmCreateGroup">创建</n-button>
      </template>
    </n-modal>

    <!-- Permission Editor -->
    <n-modal v-model:show="showPermEditor" preset="dialog" :title="`组「${editingGroupName}」的权限`" style="width: 640px;">
      <div style="padding: 8px 0;">
        <n-spin :show="permLoading">
          <div v-if="editingGroupPerms.length === 0" class="empty-note">该组暂无权限</div>
          <div v-else class="perm-list">
            <div v-for="perm in editingGroupPerms" :key="perm" class="perm-item">
              <code>{{ perm }}</code>
              <n-button v-if="authStore.isAdmin" text type="error" size="small" @click="handleRemovePerm(perm)">
                移除
              </n-button>
            </div>
          </div>
          <div v-if="authStore.isAdmin" class="perm-add" style="margin-top: 12px;">
            <n-input-group>
              <n-input v-model:value="newPermission" placeholder="输入权限节点，如 tshock.admin.kick" @keydown.enter="handleAddPerm" />
              <n-button type="primary" @click="handleAddPerm" :disabled="!newPermission.trim()">添加</n-button>
            </n-input-group>
          </div>
        </n-spin>
      </div>
      <template #action>
        <n-button @click="showPermEditor = false">关闭</n-button>
      </template>
    </n-modal>

    <!-- SSC Character Detail -->
    <n-modal v-model:show="showSscDetail" preset="dialog" :title="`SSC 角色详情 - ${sscDetailData?.username || sscDetailData?.account}`" style="width: 640px;">
      <div style="padding: 8px 0;">
        <n-spin :show="sscDetailLoading">
          <div v-if="sscDetailData" class="ssc-detail">
            <div class="detail-row"><strong>账号 ID：</strong>{{ sscDetailData.account }}</div>
            <div class="detail-row"><strong>用户名：</strong>{{ sscDetailData.username || '未知' }}</div>
            <div class="detail-row"><strong>生命值：</strong>{{ sscDetailData.health }} / {{ sscDetailData.max_health }}</div>
            <div class="detail-row"><strong>魔力值：</strong>{{ sscDetailData.mana }} / {{ sscDetailData.max_mana }}</div>
            <div class="detail-row"><strong>已完成任务：</strong>{{ sscDetailData.quests_completed }}</div>
            <div class="detail-row"><strong>出生点：</strong>{{ sscDetailData.spawn_x ?? '-' }}, {{ sscDetailData.spawn_y ?? '-' }}</div>
            <div v-if="sscDetailData.inventory" class="detail-row">
              <strong>背包数据：</strong>
              <n-button size="tiny" @click="downloadSscJson(sscDetailData)">导出 JSON</n-button>
            </div>
          </div>
        </n-spin>
      </div>
      <template #action>
        <n-button @click="showSscDetail = false">关闭</n-button>
      </template>
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, h } from 'vue'
import {
  NButton, NTag, NDataTable, NModal, NSelect, NInput, NInputGroup,
  NTabs, NTabPane, NAlert, NSpin, NFormItem, useDialog,
  type DataTableColumns
} from 'naive-ui'
import { useAuthStore } from '../../stores/auth'
import { useNotification } from '../../composables/useNotification'
import {
  serverApi,
  type TShockSecurityOverview,
  type TShockUserAccount,
  type TShockGroupSummary,
  type TShockGroupDetail,
  type TShockSscCharacterSummary,
  type TShockSscCharacter,
} from '../../api/server'

const props = defineProps<{ serverId: string }>()

const authStore = useAuthStore()
const notification = useNotification()
const dialog = useDialog()

// ─── Overview State ───
const overview = ref<TShockSecurityOverview | null>(null)
const overviewLoading = ref(false)

// ─── User editing ───
const showChangeGroup = ref(false)
const editingUser = ref('')
const selectedGroup = ref<string | null>(null)
const changingGroup = ref(false)

// ─── Group editing ───
const showCreateGroup = ref(false)
const newGroupName = ref('')
const newGroupParent = ref<string | null>(null)
const creatingGroup = ref(false)

// ─── Permission editing ───
const showPermEditor = ref(false)
const editingGroupName = ref('')
const editingGroupPerms = ref<string[]>([])
const permLoading = ref(false)
const newPermission = ref('')

// ─── SSC ───
const sscCharacters = ref<TShockSscCharacterSummary[]>([])
const sscLoading = ref(false)
const sscBackupLoading = ref(false)
const showSscDetail = ref(false)
const sscDetailData = ref<TShockSscCharacter | null>(null)
const sscDetailLoading = ref(false)

const groupOptions = computed(() =>
  (overview.value?.groups || []).map(g => ({ label: g.name, value: g.name }))
)

// ─── Table Columns ───

const userColumns = computed<DataTableColumns<TShockUserAccount>>(() => [
  { title: '用户名', key: 'username', sorter: 'default' },
  {
    title: '所属组',
    key: 'group_name',
    render(row) {
      return h('span', {}, row.group_name || '未分组')
    }
  },
  {
    title: '标签',
    key: 'tags',
    render(row) {
      const tags = []
      if (row.is_superadmin) tags.push(h(NTag, { size: 'small', type: 'error' }, { default: () => 'superadmin' }))
      if (row.ignores_ssc) tags.push(h(NTag, { size: 'small', type: 'warning' }, { default: () => '绕过 SSC' }))
      return h('div', { style: 'display:flex;gap:4px;flex-wrap:wrap;' }, tags)
    }
  },
  ...(authStore.isAdmin ? [{
    title: '操作',
    key: 'actions',
    width: 160,
    render(row: TShockUserAccount) {
      return h('div', { style: 'display:flex;gap:6px;' }, [
        h(NButton, { size: 'small', type: 'primary', text: true, onClick: () => openChangeGroup(row.username, row.group_name) }, { default: () => '改组' }),
        h(NButton, { size: 'small', type: 'error', text: true, onClick: () => confirmDeleteUser(row.username) }, { default: () => '删除' }),
      ])
    }
  }] as DataTableColumns<TShockUserAccount> : []),
])

const groupColumns = computed<DataTableColumns<TShockGroupSummary>>(() => [
  { title: '组名', key: 'name', sorter: 'default' },
  { title: '权限数', key: 'permission_count', sorter: 'default' },
  {
    title: '标签',
    key: 'tags',
    render(row) {
      const tags = []
      if (row.is_registration_group) tags.push(h(NTag, { size: 'small', type: 'info' }, { default: () => '默认注册组' }))
      if (row.is_guest_group) tags.push(h(NTag, { size: 'small' }, { default: () => '默认游客组' }))
      if (row.ignores_ssc) tags.push(h(NTag, { size: 'small', type: 'warning' }, { default: () => '含 tshock.ignore.ssc' }))
      return h('div', { style: 'display:flex;gap:4px;flex-wrap:wrap;' }, tags)
    }
  },
  {
    title: '操作',
    key: 'actions',
    width: 160,
    render(row: TShockGroupSummary) {
      const buttons = [
        h(NButton, { size: 'small', type: 'primary', text: true, onClick: () => openPermEditor(row.name) }, { default: () => '查看权限' }),
      ]
      if (authStore.isAdmin && !['superadmin', 'owner', 'guest', 'default'].includes(row.name.toLowerCase())) {
        buttons.push(
          h(NButton, { size: 'small', type: 'error', text: true, onClick: () => confirmDeleteGroup(row.name) }, { default: () => '删除' })
        )
      }
      return h('div', { style: 'display:flex;gap:6px;' }, buttons)
    }
  },
])

const sscColumns = computed<DataTableColumns<TShockSscCharacterSummary>>(() => [
  { title: '用户名', key: 'username', render(row) { return row.username || `#${row.account}` } },
  { title: '生命', key: 'health', render(row) { return `${row.health}/${row.max_health}` } },
  { title: '魔力', key: 'mana', render(row) { return `${row.mana}/${row.max_mana}` } },
  { title: '任务', key: 'quests_completed' },
  {
    title: '操作',
    key: 'actions',
    width: 120,
    render(row: TShockSscCharacterSummary) {
      return h(NButton, { size: 'small', type: 'primary', text: true, onClick: () => openSscDetail(row.account) }, { default: () => '查看/导出' })
    }
  },
])

// ─── Data Loading ───

async function loadOverview() {
  overviewLoading.value = true
  try {
    const response = await serverApi.getTshockSecurity(props.serverId)
    overview.value = response.data
  } catch (error: any) {
    notification.error('加载 TShock 权限信息失败', error?.response?.data?.error || '')
  } finally {
    overviewLoading.value = false
  }
}

async function loadSscCharacters() {
  sscLoading.value = true
  try {
    const response = await serverApi.listSscCharacters(props.serverId)
    sscCharacters.value = response.data
  } catch (error: any) {
    // Silently handle if SSC not available
    sscCharacters.value = []
  } finally {
    sscLoading.value = false
  }
}

function loadAll() {
  loadOverview()
  loadSscCharacters()
}

// ─── User Actions ───

function openChangeGroup(username: string, currentGroup?: string) {
  editingUser.value = username
  selectedGroup.value = currentGroup || null
  showChangeGroup.value = true
}

async function confirmChangeGroup() {
  if (!selectedGroup.value) return
  changingGroup.value = true
  try {
    await serverApi.updateTshockUserGroup(props.serverId, editingUser.value, selectedGroup.value)
    notification.success('用户组已更新', `${editingUser.value} → ${selectedGroup.value}`)
    showChangeGroup.value = false
    loadOverview()
  } catch (error: any) {
    notification.error('修改失败', error?.response?.data?.error || '')
  } finally {
    changingGroup.value = false
  }
}

function confirmDeleteUser(username: string) {
  dialog.error({
    title: '删除 TShock 用户',
    content: `确定要删除用户「${username}」吗？该操作不可恢复。`,
    positiveText: '删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      try {
        await serverApi.deleteTshockUser(props.serverId, username)
        notification.success('用户已删除', username)
        loadOverview()
      } catch (error: any) {
        notification.error('删除失败', error?.response?.data?.error || '')
      }
    }
  })
}

// ─── Group Actions ───

async function confirmCreateGroup() {
  if (!newGroupName.value.trim()) return
  creatingGroup.value = true
  try {
    await serverApi.createTshockGroup(props.serverId, newGroupName.value.trim(), newGroupParent.value || undefined)
    notification.success('组已创建', newGroupName.value)
    showCreateGroup.value = false
    newGroupName.value = ''
    newGroupParent.value = null
    loadOverview()
  } catch (error: any) {
    notification.error('创建失败', error?.response?.data?.error || '')
  } finally {
    creatingGroup.value = false
  }
}

function confirmDeleteGroup(name: string) {
  dialog.error({
    title: '删除 TShock 组',
    content: `确定要删除组「${name}」吗？该组下的所有权限也将被清除。`,
    positiveText: '删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      try {
        await serverApi.deleteTshockGroup(props.serverId, name)
        notification.success('组已删除', name)
        loadOverview()
      } catch (error: any) {
        notification.error('删除失败', error?.response?.data?.error || '')
      }
    }
  })
}

// ─── Permission Actions ───

async function openPermEditor(groupName: string) {
  editingGroupName.value = groupName
  showPermEditor.value = true
  permLoading.value = true
  newPermission.value = ''
  try {
    const response = await serverApi.getTshockGroup(props.serverId, groupName)
    editingGroupPerms.value = response.data.permissions
  } catch (error: any) {
    notification.error('加载权限失败', error?.response?.data?.error || '')
    editingGroupPerms.value = []
  } finally {
    permLoading.value = false
  }
}

async function handleAddPerm() {
  const perm = newPermission.value.trim()
  if (!perm) return
  try {
    await serverApi.addTshockPermission(props.serverId, editingGroupName.value, perm)
    editingGroupPerms.value.push(perm)
    editingGroupPerms.value.sort()
    newPermission.value = ''
    notification.success('权限已添加', perm)
    loadOverview() // refresh counts
  } catch (error: any) {
    notification.error('添加失败', error?.response?.data?.error || '')
  }
}

async function handleRemovePerm(perm: string) {
  try {
    await serverApi.removeTshockPermission(props.serverId, editingGroupName.value, perm)
    editingGroupPerms.value = editingGroupPerms.value.filter(p => p !== perm)
    notification.success('权限已移除', perm)
    loadOverview()
  } catch (error: any) {
    notification.error('移除失败', error?.response?.data?.error || '')
  }
}

// ─── SSC Actions ───

async function openSscDetail(accountId: number) {
  showSscDetail.value = true
  sscDetailLoading.value = true
  sscDetailData.value = null
  try {
    const response = await serverApi.exportSscCharacter(props.serverId, accountId)
    sscDetailData.value = response.data
  } catch (error: any) {
    notification.error('加载角色数据失败', error?.response?.data?.error || '')
  } finally {
    sscDetailLoading.value = false
  }
}

function downloadSscJson(data: TShockSscCharacter) {
  const json = JSON.stringify(data, null, 2)
  const blob = new Blob([json], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `ssc-character-${data.username || data.account}.json`
  a.click()
  URL.revokeObjectURL(url)
}

async function handleBackupSsc() {
  sscBackupLoading.value = true
  try {
    const response = await serverApi.backupSscCharacters(props.serverId)
    const data = response.data as any
    notification.success('SSC 角色备份完成', `已备份 ${data.character_count} 个角色`)
  } catch (error: any) {
    notification.error('备份失败', error?.response?.data?.error || '')
  } finally {
    sscBackupLoading.value = false
  }
}

onMounted(() => {
  loadAll()
})

defineExpose({ loadAll })
</script>

<style scoped>
.tshock-manager {
  background-color: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 20px;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.section-header h3 {
  margin: 0;
  color: var(--text-primary);
}

.security-summary {
  margin-bottom: 8px;
}

.summary-line {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  margin-bottom: 6px;
}

.summary-line:last-child {
  margin-bottom: 0;
}

.sub-section {
  padding: 8px 0;
}

.sub-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.perm-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  max-height: 400px;
  overflow-y: auto;
}

.perm-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 10px;
  border-radius: 6px;
  background: var(--bg-body);
  border: 1px solid var(--border-color);
}

.perm-item code {
  font-size: 13px;
  color: var(--text-primary);
}

.ssc-detail {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.detail-row {
  display: flex;
  gap: 8px;
  align-items: center;
}

.muted,
.empty-note {
  color: var(--text-muted);
  font-size: 13px;
}
</style>

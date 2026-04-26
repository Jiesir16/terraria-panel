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
            <div class="sub-header">
              <span class="muted">TShock 账号用于 /login；用户组权限不依赖 SSC。</span>
              <n-button v-if="authStore.isAdmin" size="small" type="primary" @click="openCreateAccount">
                + 新建账号
              </n-button>
            </div>
            <n-alert v-if="!authStore.isAdmin" type="warning" :show-icon="false" style="margin-bottom: 12px;">
              当前面板账号不是管理员，所以不会显示 TShock 用户改组按钮。需要面板 admin 才能修改 TShock 用户组。
            </n-alert>
            <div v-else class="quick-group-change">
              <n-select
                v-model:value="quickGroupUser"
                :options="userOptions"
                filterable
                tag
                clearable
                placeholder="TShock 账号名（可直接输入）"
                style="min-width: 220px;"
              />
              <n-select
                v-model:value="quickGroupTarget"
                :options="groupOptions"
                filterable
                clearable
                placeholder="目标用户组"
                style="min-width: 180px;"
              />
              <n-button
                size="small"
                type="primary"
                :loading="changingGroup"
                :disabled="!quickGroupUser || !quickGroupTarget"
                @click="confirmQuickChangeGroup"
              >
                修改用户组
              </n-button>
            </div>
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
              <span class="muted">TShock 组列表优先来自 REST 运行时；REST 不可用时退回 tshock.sqlite</span>
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
              <div style="display:flex;gap:8px;flex-wrap:wrap;">
                <n-button v-if="authStore.isAdmin" size="small" type="primary" @click="openCreateAccount">
                  + 新建SSC账号
                </n-button>
                <n-button v-if="authStore.isOperator" size="small" type="warning" @click="handleBackupSsc" :loading="sscBackupLoading">
                  备份所有角色
                </n-button>
              </div>
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

    <!-- Create / Edit TShock Account -->
    <n-modal v-model:show="showAccountModal" preset="dialog" :title="accountModalMode === 'create' ? '新建 TShock / SSC 账号' : `编辑账号「${accountForm.username}」`">
      <div style="padding: 8px 0;">
        <n-alert type="info" :show-icon="false" style="margin-bottom: 12px;">
          SSC 角色数据归属于 TShock 账号。新建账号后，玩家仍需要用该账号登录；角色数据会在玩家进入并保存后生成。
        </n-alert>
        <n-form-item label="账号名" :show-feedback="false" style="margin-bottom: 12px;">
          <n-input v-model:value="accountForm.username" :disabled="accountModalMode === 'edit'" placeholder="通常填玩家角色名/登录名" />
        </n-form-item>
        <n-form-item :label="accountModalMode === 'create' ? '密码' : '新密码（留空不改）'" :show-feedback="false" style="margin-bottom: 12px;">
          <n-input v-model:value="accountForm.password" type="password" show-password-on="click" placeholder="输入密码" />
        </n-form-item>
        <n-form-item label="所属组" :show-feedback="false">
          <n-select
            v-model:value="accountForm.group"
            :options="groupOptions"
            placeholder="选择组"
            clearable
            filterable
          />
        </n-form-item>
      </div>
      <template #action>
        <n-button @click="showAccountModal = false">取消</n-button>
        <n-button type="primary" :loading="accountSaving" @click="saveAccount">
          {{ accountModalMode === 'create' ? '创建' : '保存' }}
        </n-button>
      </template>
    </n-modal>

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

    <!-- Permission Tree Editor -->
    <n-modal v-model:show="showPermEditor" preset="card" :title="`编辑组「${editingGroupName}」的权限`" style="width: 720px; max-width: 95vw;" :segmented="{ content: true, footer: true }">
      <n-spin :show="permLoading || permSaving">
        <!-- Search / filter -->
        <n-input
          v-model:value="permSearchText"
          placeholder="搜索权限节点..."
          clearable
          size="small"
          style="margin-bottom: 12px;"
        />

        <!-- Permission stats -->
        <div class="perm-stats">
          <n-tag size="small" type="success">已选 {{ checkedPermKeys.length }} 项</n-tag>
          <n-tag size="small">共 {{ allLeafCount }} 项可选</n-tag>
          <n-tag v-if="permissionDirty" size="small" type="warning">有未保存修改</n-tag>
          <span v-if="authStore.isAdmin" class="muted" style="margin-left: 8px;">勾选后先暂存，点击保存后生效</span>
          <span v-if="extraPerms.length > 0" class="muted" style="margin-left: 8px;">
            + {{ extraPerms.length }} 项自定义/插件权限
          </span>
        </div>

        <!-- The tree -->
        <div class="perm-tree-container">
          <n-tree
            :data="filteredTreeData"
            :checked-keys="checkedPermKeys"
            :expanded-keys="expandedKeys"
            checkable
            selectable
            cascade
            :check-on-click="authStore.isAdmin"
            :disabled="!authStore.isAdmin"
            key-field="key"
            label-field="label"
            children-field="children"
            :render-label="renderTreeLabel"
            block-line
            virtual-scroll
            style="max-height: 420px;"
            @update:checked-keys="handleTreeCheck"
            @update:expanded-keys="handleTreeExpand"
          />
        </div>

        <!-- Extra / custom permissions (not in the tree) -->
        <div v-if="extraPerms.length > 0" class="extra-perms">
          <div class="extra-perms-title">自定义 / 插件权限（不在标准权限树中）</div>
          <div class="perm-tag-list">
            <n-tag
              v-for="perm in extraPerms"
              :key="perm"
              size="small"
              :closable="authStore.isAdmin"
              @close="handleRemoveExtraPermLocal(perm)"
            >
              {{ perm }}
            </n-tag>
          </div>
        </div>

        <!-- Manual add -->
        <div v-if="authStore.isAdmin" class="perm-add">
          <n-input-group>
            <n-input v-model:value="newPermission" placeholder="手动添加权限节点（插件权限等）" size="small" @keydown.enter="handleAddCustomPerm" />
            <n-button type="primary" size="small" @click="handleAddCustomPerm" :disabled="!newPermission.trim()">添加</n-button>
          </n-input-group>
        </div>
      </n-spin>

      <template #footer>
        <div style="display: flex; justify-content: flex-end; gap: 8px;">
          <n-button @click="handleClosePermEditor">关闭</n-button>
          <n-button
            v-if="authStore.isAdmin"
            type="primary"
            :loading="permSaving"
            :disabled="!permissionDirty"
            @click="savePermissionChanges"
          >
            保存修改
          </n-button>
        </div>
      </template>
    </n-modal>

    <!-- SSC Character Detail -->
    <n-modal v-model:show="showSscDetail" preset="dialog" :title="`SSC 角色详情 - ${sscDetailData?.username || sscDetailData?.account}`" style="width: 640px;">
      <div style="padding: 8px 0;">
        <n-spin :show="sscDetailLoading">
          <div v-if="sscDetailData" class="ssc-detail">
            <div class="detail-row"><strong>账号 ID：</strong>{{ sscDetailData.account }}</div>
            <div class="detail-row"><strong>用户名：</strong>{{ sscDetailData.username || '未知' }}</div>
            <div class="ssc-edit-grid">
              <n-form-item label="当前生命" :show-feedback="false">
                <n-input-number v-model:value="sscEditForm.health" :min="1" :max="9999" />
              </n-form-item>
              <n-form-item label="最大生命" :show-feedback="false">
                <n-input-number v-model:value="sscEditForm.max_health" :min="1" :max="9999" />
              </n-form-item>
              <n-form-item label="当前魔力" :show-feedback="false">
                <n-input-number v-model:value="sscEditForm.mana" :min="0" :max="9999" />
              </n-form-item>
              <n-form-item label="最大魔力" :show-feedback="false">
                <n-input-number v-model:value="sscEditForm.max_mana" :min="0" :max="9999" />
              </n-form-item>
              <n-form-item label="渔夫任务" :show-feedback="false">
                <n-input-number v-model:value="sscEditForm.quests_completed" :min="0" :max="9999" />
              </n-form-item>
            </div>
            <div class="detail-row"><strong>出生点：</strong>{{ sscDetailData.spawn_x ?? '-' }}, {{ sscDetailData.spawn_y ?? '-' }}</div>
            <div v-if="sscDetailData.inventory" class="detail-row">
              <strong>背包数据：</strong>
              <n-button size="tiny" @click="downloadSscJson(sscDetailData)">导出 JSON</n-button>
            </div>
          </div>
        </n-spin>
      </div>
      <template #action>
        <n-button v-if="authStore.isAdmin && sscDetailData" type="error" :loading="sscDeleting" @click="confirmDeleteSscCharacter(sscDetailData.account)">删除角色数据</n-button>
        <n-button v-if="authStore.isAdmin && sscDetailData" type="primary" :loading="sscSaving" @click="saveSscCharacter">保存角色数据</n-button>
        <n-button @click="showSscDetail = false">关闭</n-button>
      </template>
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, h, watch } from 'vue'
import {
  NButton, NTag, NDataTable, NModal, NSelect, NInput, NInputGroup,
  NTabs, NTabPane, NAlert, NSpin, NFormItem, NTree, NInputNumber, useDialog,
  type DataTableColumns, type TreeOption
} from 'naive-ui'
import { useAuthStore } from '../../stores/auth'
import { useNotification } from '../../composables/useNotification'
import {
  serverApi,
  type TShockSecurityOverview,
  type TShockUserAccount,
  type TShockGroupSummary,
  type TShockSscCharacterSummary,
  type TShockSscCharacter,
} from '../../api/server'
import { tshockRestApi } from '../../api/tshockRest'
import {
  TSHOCK_PERMISSION_TREE,
  ALL_PERMISSION_KEYS,
  type PermissionNode,
} from '../../constants/tshockPermissions'
import { PERMISSION_DESCRIPTIONS } from '../../constants/tshockPermissionDescriptions'

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
const quickGroupUser = ref<string | null>(null)
const quickGroupTarget = ref<string | null>(null)

// ─── Account editing ───
const showAccountModal = ref(false)
const accountModalMode = ref<'create' | 'edit'>('create')
const accountSaving = ref(false)
const accountForm = ref({
  username: '',
  password: '',
  group: null as string | null,
})

// ─── Group editing ───
const showCreateGroup = ref(false)
const newGroupName = ref('')
const newGroupParent = ref<string | null>(null)
const creatingGroup = ref(false)

// ─── Permission editing ───
const showPermEditor = ref(false)
const editingGroupName = ref('')
const originalGroupPerms = ref<string[]>([])
const editingGroupPerms = ref<string[]>([])
const permLoading = ref(false)
const newPermission = ref('')
const permSearchText = ref('')
const expandedKeys = ref<string[]>([])
const permSaving = ref(false)

// ─── SSC ───
const sscCharacters = ref<TShockSscCharacterSummary[]>([])
const sscLoading = ref(false)
const sscBackupLoading = ref(false)
const showSscDetail = ref(false)
const sscDetailData = ref<TShockSscCharacter | null>(null)
const sscDetailLoading = ref(false)
const sscSaving = ref(false)
const sscDeleting = ref(false)
const sscEditForm = ref({
  health: 100,
  max_health: 100,
  mana: 20,
  max_mana: 20,
  quests_completed: 0,
})

const groupOptions = computed(() =>
  (overview.value?.groups || []).map(g => ({ label: g.name, value: g.name }))
)

const userOptions = computed(() =>
  (overview.value?.users || []).map(user => ({
    label: `${user.username}${user.group_name ? ` (${user.group_name})` : ''}`,
    value: user.username,
  }))
)

function flattenPermissionLabels(nodes: PermissionNode[], target = new Map<string, string>()) {
  for (const node of nodes) {
    if (node.children?.length) {
      flattenPermissionLabels(node.children, target)
    } else {
      target.set(node.key, node.label)
    }
  }
  return target
}

const permissionLabels = flattenPermissionLabels(TSHOCK_PERMISSION_TREE)

const groupNameDescriptions: Record<string, string> = {
  guest: '未注册或未登录玩家进入服务器时使用的游客组，通常只保留聊天、注册、登录等最低权限。',
  default: '普通注册玩家的默认组，通常用于正式玩家的基础游玩权限。',
  vip: 'VIP/赞助/可信玩家组，通常继承 default，并额外开放传送、便利命令或活动权限。',
  'insecure-guest': 'TShock 的受限游客组，通常用于不安全连接、未认证或需要额外限制的访客场景。',
  newadmin: '新管理员/初级管理组，通常给少量管理命令，但不应等同于全权限。',
  admin: '管理员组，通常具备踢人、封禁、广播、传送、世界管理等主要管理能力。',
  trustedadmin: '可信管理员组，通常比 admin 权限更多，接近服主但仍应避免直接给全权限。',
  owner: '服主组，常由 setup 或面板初始化创建。通常用于长期服主账号，建议明确排除 tshock.ignore.ssc。',
  superadmin: 'TShock 最高权限组，默认等同全权限。注意：superadmin 通常会绕过 SSC，不适合作为普通游玩账号。',
}

function describeGroup(row: TShockGroupSummary) {
  const lower = row.name.toLowerCase()
  const base = groupNameDescriptions[lower] || '自定义权限组，具体能力由直接权限、否定权限和父组继承共同决定。'
  const parent = row.parent ? `父组：${row.parent}。` : '无父组。'
  const capabilities = describePermissions(row.permissions || [])
  return `${base} ${parent}${capabilities}`
}

function describePermissions(permissions: string[]) {
  if (permissions.includes('*')) return '包含 *，等同全权限。'
  if (permissions.length === 0) return '当前未读取到直接权限，可能完全依赖父组继承。'

  const positive = permissions.filter(p => !p.startsWith('!'))
  const negative = permissions.filter(p => p.startsWith('!')).map(p => p.slice(1))
  const labels = positive.slice(0, 5).map(p => permissionLabels.get(p) || p)
  const blocked = negative.slice(0, 3).map(p => permissionLabels.get(p) || p)
  const parts = [`直接权限：${labels.join('、')}${positive.length > 5 ? ` 等 ${positive.length} 项` : ''}。`]
  if (blocked.length > 0) {
    parts.push(`显式禁用：${blocked.join('、')}${negative.length > 3 ? ` 等 ${negative.length} 项` : ''}。`)
  }
  return parts.join('')
}

// ─── Permission Tree Logic ───

const allLeafKeys = new Set(ALL_PERMISSION_KEYS)
const allLeafCount = ALL_PERMISSION_KEYS.length

function normalizePermissions(permissions: string[]): string[] {
  return [...new Set(permissions.map(p => p.trim()).filter(Boolean))].sort()
}

/** Currently checked leaf keys (intersection with the tree) */
const checkedPermKeys = computed(() =>
  editingGroupPerms.value.filter(p => allLeafKeys.has(p))
)

const permissionDirty = computed(() => {
  const current = normalizePermissions(editingGroupPerms.value)
  const original = normalizePermissions(originalGroupPerms.value)
  if (current.length !== original.length) return true
  return current.some((permission, index) => permission !== original[index])
})

/** Permissions that are in the group but NOT in the standard tree (plugin perms etc.) */
const extraPerms = computed(() =>
  editingGroupPerms.value.filter(p => !allLeafKeys.has(p)).sort()
)

function permissionSearchText(node: PermissionNode) {
  const doc = PERMISSION_DESCRIPTIONS[node.key]
  return [
    node.key,
    node.label,
    doc?.descriptionZh,
    doc?.descriptionEn,
    doc?.commands,
  ].filter(Boolean).join(' ').toLowerCase()
}

function renderPermissionTreeLabel(node: PermissionNode) {
  const doc = PERMISSION_DESCRIPTIONS[node.key]
  const isLeaf = !node.children?.length

  return h('div', { class: 'permission-tree-label' }, [
    h('div', { class: 'permission-tree-main' }, [
      h('span', { class: 'permission-tree-title' }, node.label),
      isLeaf ? h('code', { class: 'permission-tree-key' }, node.key) : null,
    ]),
    doc?.descriptionZh ? h('div', { class: 'permission-tree-desc zh' }, doc.descriptionZh) : null,
    doc ? h('div', { class: 'permission-tree-desc' }, doc.descriptionEn) : null,
    doc?.commands && doc.commands !== 'None'
      ? h('div', { class: 'permission-tree-commands' }, `Commands: ${doc.commands}`)
      : null,
  ])
}

function renderTreeLabel({ option }: { option: TreeOption }) {
  const node = (option as any).permissionNode as PermissionNode | undefined
  return node ? renderPermissionTreeLabel(node) : String(option.label || '')
}

/** Convert PermissionNode[] to NTree TreeOption[], with search filtering */
function toTreeOptions(nodes: PermissionNode[], filter: string): TreeOption[] {
  const result: TreeOption[] = []
  for (const node of nodes) {
    if (node.children && node.children.length > 0) {
      const children = toTreeOptions(node.children, filter)
      // If filter active and no children match, skip this branch
      if (filter && children.length === 0 && !permissionSearchText(node).includes(filter)) continue
      result.push({
        key: node.key,
        label: node.label,
        permissionNode: node,
        children,
      } as TreeOption)
    } else {
      // Leaf node — apply filter
      if (filter && !permissionSearchText(node).includes(filter)) continue
      result.push({
        key: node.key,
        label: node.label,
        permissionNode: node,
      } as TreeOption)
    }
  }
  return result
}

const filteredTreeData = computed(() => {
  const filter = permSearchText.value.trim().toLowerCase()
  return toTreeOptions(TSHOCK_PERMISSION_TREE, filter)
})

// Auto-expand all when searching
watch(permSearchText, (val) => {
  if (val.trim()) {
    // Expand all category keys so search results are visible
    const allCategoryKeys = collectCategoryKeys(TSHOCK_PERMISSION_TREE)
    expandedKeys.value = allCategoryKeys
  }
})

function collectCategoryKeys(nodes: PermissionNode[]): string[] {
  const keys: string[] = []
  for (const node of nodes) {
    if (node.children && node.children.length > 0) {
      keys.push(node.key)
      keys.push(...collectCategoryKeys(node.children))
    }
  }
  return keys
}

function handleTreeExpand(keys: string[]) {
  expandedKeys.value = keys
}

/** When user checks/unchecks tree nodes, update local draft only. */
function handleTreeCheck(newCheckedKeys: string[]) {
  if (!authStore.isAdmin) return

  const newLeaves = newCheckedKeys.filter(k => allLeafKeys.has(k))
  const extra = editingGroupPerms.value.filter(p => !allLeafKeys.has(p))
  editingGroupPerms.value = normalizePermissions([...extra, ...newLeaves])
}

function handleRemoveExtraPermLocal(perm: string) {
  editingGroupPerms.value = editingGroupPerms.value.filter(p => p !== perm)
}

function handleAddCustomPerm() {
  const perm = newPermission.value.trim()
  if (!perm) return
  if (editingGroupPerms.value.includes(perm)) {
    notification.error('权限已存在', perm)
    return
  }
  editingGroupPerms.value = normalizePermissions([...editingGroupPerms.value, perm])
  newPermission.value = ''
}

async function savePermissionChanges() {
  if (!authStore.isAdmin || !permissionDirty.value) return

  const original = normalizePermissions(originalGroupPerms.value)
  const current = normalizePermissions(editingGroupPerms.value)
  const originalSet = new Set(original)
  const currentSet = new Set(current)
  const toAdd = current.filter(permission => !originalSet.has(permission))
  const toRemove = original.filter(permission => !currentSet.has(permission))

  permSaving.value = true
  try {
    await tshockRestApi.groupUpdate(
      props.serverId,
      editingGroupName.value,
      undefined,
      current.join(',')
    )

    originalGroupPerms.value = current
    editingGroupPerms.value = current
    const msg = []
    if (toAdd.length > 0) msg.push(`新增 ${toAdd.length}`)
    if (toRemove.length > 0) msg.push(`移除 ${toRemove.length}`)
    notification.success('权限已保存', msg.join('，') || '没有变化')
    await loadOverview()
  } catch (error: any) {
    notification.error('保存权限失败', error?.response?.data?.error || '部分权限可能未生效，请刷新重试')
    await openPermEditor(editingGroupName.value)
  } finally {
    permSaving.value = false
  }
}

function handleClosePermEditor() {
  if (!permissionDirty.value) {
    showPermEditor.value = false
    return
  }

  dialog.warning({
    title: '存在未保存修改',
    content: '关闭后会丢弃当前权限修改。',
    positiveText: '丢弃并关闭',
    negativeText: '继续编辑',
    onPositiveClick: () => {
      showPermEditor.value = false
    }
  })
}

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
        h(NButton, { size: 'small', type: 'primary', text: true, onClick: () => openEditAccount(row) }, { default: () => '编辑' }),
        h(NButton, { size: 'small', type: 'primary', text: true, onClick: () => openChangeGroup(row.username, row.group_name) }, { default: () => '改组' }),
        h(NButton, { size: 'small', type: 'error', text: true, onClick: () => confirmDeleteUser(row.username) }, { default: () => '删除' }),
      ])
    }
  }] as DataTableColumns<TShockUserAccount> : []),
])

function groupRestUrl(name: string) {
  return `/api/servers/${props.serverId}/rest/groups/${encodeURIComponent(name)}`
}

const groupColumns = computed<DataTableColumns<TShockGroupSummary>>(() => [
  { title: '组名', key: 'name', sorter: 'default' },
  { title: '父组', key: 'parent', render(row) { return row.parent || '-' } },
  {
    title: '用途说明',
    key: 'description',
    ellipsis: { tooltip: true },
    render(row) {
      return describeGroup(row)
    }
  },
  { title: '权限数', key: 'permission_count', sorter: 'default' },
  {
    title: '权限预览',
    key: 'permissions',
    ellipsis: { tooltip: true },
    render(row) {
      const permissions = row.permissions || []
      if (permissions.length === 0) return '无'
      const preview = permissions.slice(0, 8).join(', ')
      return permissions.length > 8 ? `${preview} ...` : preview
    }
  },
  {
    title: 'REST URL',
    key: 'url',
    ellipsis: { tooltip: true },
    render(row) {
      return h('code', { class: 'url-code' }, groupRestUrl(row.name))
    }
  },
  {
    title: '标签',
    key: 'tags',
    render(row) {
      const tags = []
      tags.push(h(NTag, { size: 'small', type: row.source === 'rest' ? 'success' : 'warning' }, { default: () => row.source === 'rest' ? 'REST' : 'SQLite' }))
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
        h(NButton, { size: 'small', type: 'primary', text: true, onClick: () => openPermEditor(row.name) }, { default: () => '编辑权限' }),
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
    width: 180,
    render(row: TShockSscCharacterSummary) {
      const buttons = [
        h(NButton, { size: 'small', type: 'primary', text: true, onClick: () => openSscDetail(row.account) }, { default: () => '查看/编辑' }),
      ]
      if (authStore.isAdmin) {
        buttons.push(h(NButton, { size: 'small', type: 'error', text: true, onClick: () => confirmDeleteSscCharacter(row.account) }, { default: () => '删除角色' }))
      }
      return h('div', { style: 'display:flex;gap:6px;' }, buttons)
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

function openCreateAccount() {
  accountModalMode.value = 'create'
  accountForm.value = {
    username: '',
    password: '',
    group: overview.value?.default_registration_group || 'default',
  }
  showAccountModal.value = true
}

function openEditAccount(row: TShockUserAccount) {
  accountModalMode.value = 'edit'
  accountForm.value = {
    username: row.username,
    password: '',
    group: row.group_name || null,
  }
  showAccountModal.value = true
}

async function saveAccount() {
  const username = accountForm.value.username.trim()
  const password = accountForm.value.password.trim()
  const group = accountForm.value.group || undefined
  if (!username) {
    notification.error('账号名不能为空', '')
    return
  }
  if (accountModalMode.value === 'create' && !password) {
    notification.error('密码不能为空', '新建账号必须设置密码')
    return
  }

  accountSaving.value = true
  try {
    if (accountModalMode.value === 'create') {
      await tshockRestApi.userCreate(props.serverId, username, password, group)
      notification.success('账号已创建', username)
    } else {
      await tshockRestApi.userUpdate(props.serverId, username, password || undefined, group)
      notification.success('账号已更新', username)
    }
    showAccountModal.value = false
    await loadOverview()
  } catch (error: any) {
    notification.error(accountModalMode.value === 'create' ? '创建账号失败' : '更新账号失败', error?.response?.data?.error || '')
  } finally {
    accountSaving.value = false
  }
}

function openChangeGroup(username: string, currentGroup?: string) {
  editingUser.value = username
  selectedGroup.value = currentGroup || null
  showChangeGroup.value = true
}

async function updateTshockUserGroup(username: string, group: string) {
  changingGroup.value = true
  try {
    await tshockRestApi.userUpdate(props.serverId, username, undefined, group)
    notification.success('用户组已更新', `${username} → ${group}`)
    await loadOverview()
  } catch (error: any) {
    notification.error('修改失败', error?.response?.data?.error || '')
  } finally {
    changingGroup.value = false
  }
}

async function confirmChangeGroup() {
  if (!selectedGroup.value) return
  await updateTshockUserGroup(editingUser.value, selectedGroup.value)
  showChangeGroup.value = false
}

async function confirmQuickChangeGroup() {
  const username = quickGroupUser.value?.trim()
  const group = quickGroupTarget.value
  if (!username || !group) return
  await updateTshockUserGroup(username, group)
}

function confirmDeleteUser(username: string) {
  dialog.error({
    title: '删除 TShock 用户',
    content: `确定要删除用户「${username}」吗？该操作不可恢复。`,
    positiveText: '删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      try {
        await tshockRestApi.userDestroy(props.serverId, username)
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
    await tshockRestApi.groupCreate(props.serverId, newGroupName.value.trim(), newGroupParent.value || undefined)
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
        await tshockRestApi.groupDestroy(props.serverId, name)
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
  permSearchText.value = ''
  newPermission.value = ''
  expandedKeys.value = []
  try {
    const response = await serverApi.getTshockGroup(props.serverId, groupName)
    const permissions = normalizePermissions(response.data.permissions)
    originalGroupPerms.value = permissions
    editingGroupPerms.value = permissions
  } catch (error: any) {
    notification.error('加载权限失败', error?.response?.data?.error || '')
    originalGroupPerms.value = []
    editingGroupPerms.value = []
  } finally {
    permLoading.value = false
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
    sscEditForm.value = {
      health: response.data.health,
      max_health: response.data.max_health,
      mana: response.data.mana,
      max_mana: response.data.max_mana,
      quests_completed: response.data.quests_completed,
    }
  } catch (error: any) {
    notification.error('加载角色数据失败', error?.response?.data?.error || '')
  } finally {
    sscDetailLoading.value = false
  }
}

async function saveSscCharacter() {
  if (!sscDetailData.value) return
  sscSaving.value = true
  try {
    await serverApi.updateSscCharacter(props.serverId, sscDetailData.value.account, {
      health: sscEditForm.value.health,
      max_health: sscEditForm.value.max_health,
      mana: sscEditForm.value.mana,
      max_mana: sscEditForm.value.max_mana,
      quests_completed: sscEditForm.value.quests_completed,
    })
    notification.success('SSC 角色数据已保存', sscDetailData.value.username || `#${sscDetailData.value.account}`)
    await openSscDetail(sscDetailData.value.account)
    await loadSscCharacters()
  } catch (error: any) {
    notification.error('保存角色数据失败', error?.response?.data?.error || '')
  } finally {
    sscSaving.value = false
  }
}

function confirmDeleteSscCharacter(accountId: number) {
  dialog.error({
    title: '删除 SSC 角色数据',
    content: `确定删除账号 ID ${accountId} 的 SSC 角色数据吗？这不会删除 TShock 账号，但会移除该账号保存的服务端角色。建议先备份。`,
    positiveText: '删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      sscDeleting.value = true
      try {
        await serverApi.deleteSscCharacter(props.serverId, accountId)
        notification.success('SSC 角色数据已删除', `账号 ID: ${accountId}`)
        showSscDetail.value = false
        await loadSscCharacters()
      } catch (error: any) {
        notification.error('删除角色数据失败', error?.response?.data?.error || '')
      } finally {
        sscDeleting.value = false
      }
    }
  })
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

.quick-group-change {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  align-items: center;
  margin-bottom: 12px;
  padding: 12px;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: var(--bg-body);
}

.perm-stats {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
}

.perm-tree-container {
  border: 1px solid var(--border-color);
  border-radius: 8px;
  padding: 8px;
  background: var(--bg-body);
}

.permission-tree-label {
  padding: 4px 0;
  line-height: 1.35;
}

.permission-tree-main {
  display: flex;
  gap: 8px;
  align-items: center;
  flex-wrap: wrap;
}

.permission-tree-title {
  font-weight: 600;
  color: var(--text-primary);
}

.permission-tree-key {
  font-size: 12px;
  color: var(--text-muted);
  background: transparent;
}

.permission-tree-desc,
.permission-tree-commands {
  margin-top: 2px;
  font-size: 12px;
  color: var(--text-muted);
}

.permission-tree-desc.zh {
  color: var(--text-primary);
}

.extra-perms {
  margin-top: 16px;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  padding: 12px;
  background: var(--bg-body);
}

.extra-perms-title {
  font-size: 13px;
  color: var(--text-muted);
  margin-bottom: 8px;
}

.perm-tag-list {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.perm-add {
  margin-top: 12px;
}

.ssc-detail {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.ssc-edit-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px 12px;
}

.detail-row {
  display: flex;
  gap: 8px;
  align-items: center;
}

.url-code {
  font-size: 12px;
  color: var(--text-muted);
  word-break: break-all;
}

.muted,
.empty-note {
  color: var(--text-muted);
  font-size: 13px;
}
</style>

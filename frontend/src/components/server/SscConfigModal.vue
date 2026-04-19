<template>
  <n-modal v-model:show="modalShow" preset="dialog" title="SSC 配置" style="width: 920px;">
    <n-spin :show="loading">
      <n-form :model="formData" label-placement="left" label-width="180px">
        <n-form-item label="启用 SSC">
          <n-checkbox v-model:checked="formData.Settings.Enabled">启用服务端存档</n-checkbox>
        </n-form-item>

        <n-form-item label="保存间隔（分钟）">
          <n-input-number v-model:value="formData.Settings.ServerSideCharacterSave" :min="1" :max="120" style="width: 100%;" />
        </n-form-item>

        <n-form-item label="登录丢弃阈值">
          <n-input-number v-model:value="formData.Settings.LogonDiscardThreshold" :min="0" :max="9999" style="width: 100%;" />
        </n-form-item>

        <n-form-item label="初始生命值">
          <n-input-number v-model:value="formData.Settings.StartingHealth" :min="100" :max="500" style="width: 100%;" />
        </n-form-item>

        <n-form-item label="初始魔力值">
          <n-input-number v-model:value="formData.Settings.StartingMana" :min="20" :max="400" style="width: 100%;" />
        </n-form-item>

        <n-form-item label="绕过提醒">
          <n-checkbox v-model:checked="formData.Settings.WarnPlayersAboutBypassPermission">
            提醒拥有绕过权限的玩家 SSC 不生效
          </n-checkbox>
        </n-form-item>

        <n-form-item label="保留外观">
          <n-checkbox v-model:checked="formData.Settings.KeepPlayerAppearance">
            保留玩家外观设置
          </n-checkbox>
        </n-form-item>

        <n-form-item label="初始物品">
          <div class="inventory-editor">
            <div class="inventory-toolbar">
              <n-select
                v-model:value="selectedItemId"
                :options="itemOptions"
                filterable
                clearable
                :loading="itemCatalogLoading"
                placeholder="从物品清单选择"
              />
              <n-input-number v-model:value="newItemStack" :min="1" :max="9999" placeholder="数量" />
              <n-button size="small" type="primary" @click="addSelectedInventoryItem" :disabled="!selectedItemId">
                添加物品
              </n-button>
              <n-button size="small" secondary @click="addInventoryItem">
                新增空行
              </n-button>
            </div>
            <div class="inventory-toolbar secondary">
              <n-input v-model:value="itemQuery" placeholder="搜索物品 ID / 名称 / 内部名" clearable @keyup.enter="loadItemCatalog" />
              <n-button size="small" @click="loadItemCatalog" :loading="itemCatalogLoading">搜索/刷新清单</n-button>
              <n-button size="small" type="warning" @click="syncItemCatalog" :loading="itemCatalogSyncing">重新下载清单</n-button>
            </div>
            <span class="field-hint">
              每行对应一个起始物品。清单缓存按当前服务器版本保存；旧的负数 netID 保留为手动/特殊 ID。
            </span>

            <div v-if="formData.Settings.StartingInventory.length === 0" class="empty-inventory">
              当前没有初始物品
            </div>

            <div v-else class="inventory-table">
              <div class="inventory-header">
                <span>netID</span>
                <span>物品名称</span>
                <span>prefix</span>
                <span>stack</span>
                <span>收藏</span>
                <span>操作</span>
              </div>

              <div
                v-for="(item, index) in formData.Settings.StartingInventory"
                :key="`${index}-${item.netID}-${item.prefix}-${item.stack}-${item.favorited}`"
                class="inventory-row"
              >
                <n-input-number v-model:value="item.netID" style="width: 100%;" />
                <span class="item-name">{{ itemName(item.netID) }}</span>
                <n-input-number v-model:value="item.prefix" :min="0" :max="255" style="width: 100%;" />
                <n-input-number v-model:value="item.stack" :min="1" :max="9999" style="width: 100%;" />
                <n-checkbox v-model:checked="item.favorited" />
                <n-button size="small" type="error" quaternary @click="removeInventoryItem(index)">
                  删除
                </n-button>
              </div>
            </div>

            <div class="catalog-summary" v-if="itemCatalogVersion">
              物品清单：{{ itemCatalogVersion }} · {{ itemCatalogSource }} · 已显示 {{ itemCatalog.length }} 条
            </div>
            <n-data-table
              v-if="itemCatalog.length > 0"
              :columns="itemCatalogColumns"
              :data="itemCatalog"
              :row-key="(row: any) => row.id"
              :pagination="{ pageSize: 8 }"
              size="small"
              striped
            />
          </div>
        </n-form-item>

        <n-alert type="info" :show-icon="false">
          这些项对应 `tshock/sscconfig.json`。保存后会同步更新 SSC 总开关；通常需要重启服务器后完全生效。
        </n-alert>
      </n-form>
    </n-spin>

    <template #action>
      <n-button @click="handleCancel">取消</n-button>
      <n-button type="primary" :loading="saving" @click="handleSave">保存</n-button>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { computed, h, ref, watch } from 'vue'
import { NModal, NSpin, NForm, NFormItem, NInputNumber, NCheckbox, NButton, NAlert, NSelect, NInput, NDataTable } from 'naive-ui'
import type { DataTableColumns } from 'naive-ui'
import { serverApi, type SscConfig, type SscInventoryItem, type TerrariaItem } from '../../api/server'
import { useNotification } from '../../composables/useNotification'

interface Props {
  show: boolean
  serverId: string
}

interface Emits {
  (e: 'update:show', value: boolean): void
  (e: 'saved', config: SscConfig): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()
const notification = useNotification()

const loading = ref(false)
const saving = ref(false)
const itemCatalogLoading = ref(false)
const itemCatalogSyncing = ref(false)
const itemCatalog = ref<TerrariaItem[]>([])
const itemCatalogVersion = ref('')
const itemCatalogSource = ref('')
const itemQuery = ref('')
const selectedItemId = ref<number | null>(null)
const newItemStack = ref(1)
const formData = ref<SscConfig>({
  Settings: {
    Enabled: false,
    ServerSideCharacterSave: 5,
    LogonDiscardThreshold: 250,
    StartingHealth: 100,
    StartingMana: 20,
    StartingInventory: [
      { netID: -15, prefix: 0, stack: 1, favorited: false },
      { netID: -13, prefix: 0, stack: 1, favorited: false },
      { netID: -16, prefix: 0, stack: 1, favorited: false }
    ],
    WarnPlayersAboutBypassPermission: true,
    KeepPlayerAppearance: false
  }
})

const modalShow = computed({
  get: () => props.show,
  set: (value: boolean) => emit('update:show', value)
})

const itemOptions = computed(() => itemCatalog.value.map((item) => ({
  label: `#${item.id} ${item.name} (${item.internal_name})`,
  value: item.id
})))

const itemById = computed(() => {
  const map = new Map<number, TerrariaItem>()
  for (const item of itemCatalog.value) {
    map.set(item.id, item)
  }
  return map
})

const itemCatalogColumns: DataTableColumns<TerrariaItem> = [
  { title: 'ID', key: 'id', width: 80 },
  { title: '名称', key: 'name', width: 180 },
  { title: '内部名', key: 'internal_name', width: 220 },
  {
    title: '操作',
    key: 'actions',
    width: 90,
    render: (row) => h(NButton, {
      size: 'tiny',
      type: selectedItemId.value === row.id ? 'primary' : 'default',
      onClick: () => {
        selectedItemId.value = row.id
      }
    }, { default: () => selectedItemId.value === row.id ? '已选择' : '选择' })
  }
]

function normalizeInventory(items: SscInventoryItem[] | undefined): SscInventoryItem[] {
  if (!Array.isArray(items)) {
    return []
  }

  return items.map((item) => ({
    netID: Number(item?.netID ?? 0),
    prefix: Number(item?.prefix ?? 0),
    stack: Math.max(1, Number(item?.stack ?? 1)),
    favorited: Boolean(item?.favorited ?? false)
  }))
}

async function loadConfig() {
  loading.value = true
  try {
    const response = await serverApi.getSscConfig(props.serverId)
    formData.value = {
      ...response.data,
      Settings: {
        ...response.data.Settings,
        StartingInventory: normalizeInventory(response.data.Settings?.StartingInventory)
      }
    }
  } catch (error: any) {
    notification.error('加载 SSC 配置失败', error?.response?.data?.error || '')
  } finally {
    loading.value = false
  }
}

async function loadItemCatalog() {
  itemCatalogLoading.value = true
  try {
    const response = await serverApi.getItemCatalog(props.serverId, itemQuery.value || undefined, 10000)
    itemCatalog.value = response.data.items || []
    itemCatalogVersion.value = response.data.version || ''
    itemCatalogSource.value = response.data.source || ''
  } catch (error: any) {
    notification.error('加载物品清单失败', error?.response?.data?.error || '')
  } finally {
    itemCatalogLoading.value = false
  }
}

async function syncItemCatalog() {
  itemCatalogSyncing.value = true
  try {
    const response = await serverApi.syncItemCatalog(props.serverId)
    itemCatalog.value = response.data.items || []
    itemCatalogVersion.value = response.data.version || ''
    itemCatalogSource.value = response.data.source || ''
    notification.success('物品清单已下载', `共 ${itemCatalog.value.length} 条`)
  } catch (error: any) {
    notification.error('下载物品清单失败', error?.response?.data?.error || '')
  } finally {
    itemCatalogSyncing.value = false
  }
}

function handleCancel() {
  modalShow.value = false
}

function itemName(id: number) {
  const item = itemById.value.get(Number(id))
  if (item) {
    return item.name
  }
  return Number(id) < 0 ? '手动/特殊 ID' : '未收录'
}

function addInventoryItem() {
  formData.value.Settings.StartingInventory.push({
    netID: 0,
    prefix: 0,
    stack: 1,
    favorited: false
  })
}

function addSelectedInventoryItem() {
  if (!selectedItemId.value) return
  formData.value.Settings.StartingInventory.push({
    netID: selectedItemId.value,
    prefix: 0,
    stack: Math.max(1, Number(newItemStack.value || 1)),
    favorited: false
  })
}

function removeInventoryItem(index: number) {
  formData.value.Settings.StartingInventory.splice(index, 1)
}

async function handleSave() {
  saving.value = true
  try {
    const payload: SscConfig = {
      ...formData.value,
      Settings: {
        ...formData.value.Settings,
        StartingInventory: normalizeInventory(formData.value.Settings.StartingInventory)
      }
    }
    await serverApi.updateSscConfig(props.serverId, payload)
    emit('saved', payload)
    notification.success('SSC 配置已保存', '已写入 sscconfig.json')
    modalShow.value = false
  } catch (error: any) {
    notification.error('保存 SSC 配置失败', error?.response?.data?.error || '')
  } finally {
    saving.value = false
  }
}

watch(
  () => props.show,
  (show) => {
    if (show) {
      loadConfig()
      loadItemCatalog()
    }
  }
)
</script>

<style scoped>
.field-hint {
  display: block;
  margin-top: 4px;
  font-size: 12px;
  color: var(--text-muted);
}

.inventory-editor {
  width: 100%;
}

.inventory-toolbar {
  display: grid;
  grid-template-columns: minmax(260px, 1fr) 110px auto auto;
  align-items: center;
  gap: 12px;
  margin-bottom: 10px;
}

.inventory-toolbar.secondary {
  grid-template-columns: minmax(260px, 1fr) auto auto;
}

.inventory-table {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 12px;
}

.inventory-header,
.inventory-row {
  display: grid;
  grid-template-columns: minmax(110px, 1fr) minmax(130px, 1.2fr) minmax(90px, 1fr) minmax(90px, 1fr) 64px 72px;
  gap: 8px;
  align-items: center;
}

.inventory-header {
  font-size: 12px;
  color: var(--text-muted);
  padding: 0 4px;
}

.empty-inventory {
  padding: 12px;
  border: 1px dashed var(--border-color);
  border-radius: 8px;
  color: var(--text-muted);
  font-size: 13px;
}

.item-name {
  color: var(--text-secondary);
  font-size: 13px;
}

.catalog-summary {
  margin: 14px 0 8px;
  color: var(--text-muted);
  font-size: 12px;
}

@media (max-width: 900px) {
  .inventory-toolbar,
  .inventory-toolbar.secondary,
  .inventory-header,
  .inventory-row {
    grid-template-columns: 1fr;
  }
}
</style>

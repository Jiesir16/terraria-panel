<template>
  <n-modal v-model:show="modalShow" preset="dialog" title="SSC 配置" style="width: 720px;">
    <n-spin :show="loading">
      <n-form :model="formData" label-placement="left" label-width="180px">
        <n-form-item label="启用 SSC">
          <n-checkbox v-model:checked="formData.Enabled">启用服务端存档</n-checkbox>
        </n-form-item>

        <n-form-item label="保存间隔（分钟）">
          <n-input-number v-model:value="formData.ServerSideCharacterSave" :min="1" :max="120" style="width: 100%;" />
        </n-form-item>

        <n-form-item label="登录丢弃阈值">
          <n-input-number v-model:value="formData.LogonDiscardThreshold" :min="0" :max="9999" style="width: 100%;" />
        </n-form-item>

        <n-form-item label="初始生命值">
          <n-input-number v-model:value="formData.StartingHealth" :min="100" :max="500" style="width: 100%;" />
        </n-form-item>

        <n-form-item label="初始魔力值">
          <n-input-number v-model:value="formData.StartingMana" :min="20" :max="400" style="width: 100%;" />
        </n-form-item>

        <n-form-item label="初始物品">
          <div class="inventory-editor">
            <div class="inventory-toolbar">
              <n-button size="small" secondary @click="addInventoryItem">
                新增物品
              </n-button>
              <span class="field-hint">每行对应一个起始物品。`netID` 为 Terraria 物品 ID，负数通常是工具。</span>
            </div>

            <div v-if="formData.StartingInventory.length === 0" class="empty-inventory">
              当前没有初始物品
            </div>

            <div v-else class="inventory-table">
              <div class="inventory-header">
                <span>netID</span>
                <span>prefix</span>
                <span>stack</span>
                <span>操作</span>
              </div>

              <div
                v-for="(item, index) in formData.StartingInventory"
                :key="`${index}-${item.netID}-${item.prefix}-${item.stack}`"
                class="inventory-row"
              >
                <n-input-number v-model:value="item.netID" style="width: 100%;" />
                <n-input-number v-model:value="item.prefix" :min="0" :max="255" style="width: 100%;" />
                <n-input-number v-model:value="item.stack" :min="1" :max="9999" style="width: 100%;" />
                <n-button size="small" type="error" quaternary @click="removeInventoryItem(index)">
                  删除
                </n-button>
              </div>
            </div>
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
import { computed, ref, watch } from 'vue'
import { NModal, NSpin, NForm, NFormItem, NInputNumber, NCheckbox, NButton, NAlert } from 'naive-ui'
import { serverApi, type SscConfig, type SscInventoryItem } from '../../api/server'
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
const formData = ref<SscConfig>({
  Enabled: false,
  ServerSideCharacterSave: 5,
  LogonDiscardThreshold: 250,
  StartingHealth: 100,
  StartingMana: 20,
  StartingInventory: [
    { netID: -15, prefix: 0, stack: 1 },
    { netID: -13, prefix: 0, stack: 1 },
    { netID: -16, prefix: 0, stack: 1 }
  ]
})

const modalShow = computed({
  get: () => props.show,
  set: (value: boolean) => emit('update:show', value)
})

function normalizeInventory(items: SscInventoryItem[] | undefined): SscInventoryItem[] {
  if (!Array.isArray(items)) {
    return []
  }

  return items.map((item) => ({
    netID: Number(item?.netID ?? 0),
    prefix: Number(item?.prefix ?? 0),
    stack: Math.max(1, Number(item?.stack ?? 1))
  }))
}

async function loadConfig() {
  loading.value = true
  try {
    const response = await serverApi.getSscConfig(props.serverId)
    formData.value = {
      ...response.data,
      StartingInventory: normalizeInventory(response.data.StartingInventory)
    }
  } catch (error: any) {
    notification.error('加载 SSC 配置失败', error?.response?.data?.error || '')
  } finally {
    loading.value = false
  }
}

function handleCancel() {
  modalShow.value = false
}

function addInventoryItem() {
  formData.value.StartingInventory.push({
    netID: 0,
    prefix: 0,
    stack: 1
  })
}

function removeInventoryItem(index: number) {
  formData.value.StartingInventory.splice(index, 1)
}

async function handleSave() {
  saving.value = true
  try {
    const payload: SscConfig = {
      ...formData.value,
      StartingInventory: normalizeInventory(formData.value.StartingInventory)
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
    }
  }
)
</script>

<style scoped>
.field-hint {
  margin-top: 4px;
  font-size: 12px;
  color: var(--text-muted);
}

.inventory-editor {
  width: 100%;
}

.inventory-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 10px;
}

.inventory-table {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.inventory-header,
.inventory-row {
  display: grid;
  grid-template-columns: minmax(120px, 1fr) minmax(100px, 1fr) minmax(100px, 1fr) 72px;
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
</style>

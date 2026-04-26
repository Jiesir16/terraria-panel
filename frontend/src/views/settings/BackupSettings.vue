<template>
  <div class="backup-settings">
    <h2>自动备份默认策略</h2>
    <n-spin :show="loading">
      <div class="form">
        <n-checkbox v-model:checked="settings.enabled">启用自动备份</n-checkbox>
        <n-input-number v-model:value="settings.interval_minutes" :min="1" :max="10080" style="width: 100%;" placeholder="默认备份间隔（分钟）" />
        <n-input-number v-model:value="settings.max_backups_per_server" :min="0" :max="10000" style="width: 100%;" placeholder="每服最多保留多少个未归档备份，0 = 不限制" />
        <n-input-number v-model:value="settings.local_retention_days" :min="0" :max="3650" style="width: 100%;" placeholder="本地保留天数，0 = 不按天清理" />
        <n-checkbox v-model:checked="settings.backup_ssc">自动备份 SSC 数据库</n-checkbox>
        <n-checkbox v-model:checked="settings.archive_daily_enabled">启用每日归档</n-checkbox>
        <n-input-number v-model:value="settings.archive_hour" :min="0" :max="23" style="width: 100%;" placeholder="每日归档执行小时" />
        <n-input-number v-model:value="settings.archive_after_days" :min="0" :max="3650" style="width: 100%;" placeholder="归档几天前的小时备份" />

        <n-divider style="margin: 8px 0;">远端同步</n-divider>

        <n-checkbox v-model:checked="settings.oss.enabled">启用远端同步</n-checkbox>

        <div v-if="settings.oss.enabled" class="oss-form">
          <div class="field-row">
            <span class="label">同步类型:</span>
            <n-select
              v-model:value="settings.oss.provider"
              :options="providerOptions"
              style="flex: 1;"
            />
          </div>

          <n-input
            v-model:value="settings.oss.prefix"
            placeholder="远端前缀（例如 terraria-panel/saves）"
          />

          <template v-if="settings.oss.provider === 'nas'">
            <n-input
              v-model:value="settings.oss.local_path"
              placeholder="NAS 挂载的本地路径（例如 /mnt/truenas-backup）"
            />
            <div class="hint">
              备份会被复制到 <code>{{ nasDestinationPreview }}</code>
            </div>
          </template>

          <template v-else-if="settings.oss.provider === 'tencent_cos'">
            <n-input v-model:value="settings.oss.bucket" placeholder="Bucket（例如 my-bucket-1250000000）" />
            <n-input v-model:value="settings.oss.region" placeholder="Region（例如 ap-shanghai）" />
            <n-input v-model:value="settings.oss.endpoint" placeholder="Endpoint（留空使用默认 <bucket>.cos.<region>.myqcloud.com）" />
            <n-input v-model:value="settings.oss.access_key_id" placeholder="SecretId" />
            <n-input
              v-model:value="settings.oss.access_key_secret"
              type="password"
              show-password-on="click"
              placeholder="SecretKey"
            />
          </template>
        </div>

        <div class="info-list" style="margin: 4px 0 8px 0;">
          <div class="info-item">
            <span class="label">当前同步:</span>
            <span class="value">{{ syncSummary }}</span>
          </div>
        </div>

        <n-button v-if="authStore.user?.role === 'admin'" type="primary" :loading="saving" @click="handleSave">
          保存备份默认策略
        </n-button>
      </div>
    </n-spin>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { NSpin, NCheckbox, NInput, NInputNumber, NButton, NSelect, NDivider } from 'naive-ui'
import { systemApi, type BackupSettings } from '../../api/system'
import { useAuthStore } from '../../stores/auth'
import { useNotification } from '../../composables/useNotification'

const authStore = useAuthStore()
const notification = useNotification()
const loading = ref(false)
const saving = ref(false)

const providerOptions = [
  { label: 'NAS / 本地目录', value: 'nas' },
  { label: '腾讯云 COS', value: 'tencent_cos' }
]

const settings = ref<BackupSettings>({
  enabled: true,
  interval_minutes: 60,
  max_backups_per_server: 0,
  local_retention_days: 30,
  backup_ssc: true,
  archive_daily_enabled: true,
  archive_hour: 1,
  archive_after_days: 2,
  oss: {
    enabled: false,
    provider: 'nas',
    endpoint: '',
    bucket: '',
    region: '',
    access_key_id: '',
    access_key_secret: '',
    local_path: '',
    prefix: 'terraria-panel/saves'
  }
})

const nasDestinationPreview = computed(() => {
  const base = settings.value.oss.local_path?.trim() || '<local_path>'
  const prefix = settings.value.oss.prefix?.trim() || ''
  return prefix ? `${base.replace(/\/$/, '')}/${prefix.replace(/^\//, '')}/<server>/<file>` : `${base.replace(/\/$/, '')}/<server>/<file>`
})

const syncSummary = computed(() => {
  const oss = settings.value.oss
  if (!oss.enabled) return '未启用'
  if (oss.provider === 'nas') return `NAS → ${oss.local_path || '(未配置路径)'}`
  if (oss.provider === 'tencent_cos') return `腾讯云 COS → ${oss.bucket || '(未配置 bucket)'}`
  return oss.provider
})

async function loadSettings() {
  loading.value = true
  try {
    const response = await systemApi.getBackupSettings()
    settings.value = response.data
  } catch {
    notification.error('加载备份默认策略失败', '')
  } finally {
    loading.value = false
  }
}

async function handleSave() {
  if (settings.value.oss.enabled) {
    if (settings.value.oss.provider === 'nas' && !settings.value.oss.local_path.trim()) {
      notification.error('NAS 路径不能为空', '请填写挂载的本地路径，例如 /mnt/truenas-backup')
      return
    }
    if (settings.value.oss.provider === 'tencent_cos') {
      const { bucket, access_key_id, access_key_secret, region, endpoint } = settings.value.oss
      if (!bucket.trim() || !access_key_id.trim() || !access_key_secret.trim()) {
        notification.error('腾讯云 COS 配置不完整', '至少需要 bucket、SecretId、SecretKey')
        return
      }
      if (!region.trim() && !endpoint.trim()) {
        notification.error('腾讯云 COS 配置不完整', 'region 和 endpoint 至少填一个')
        return
      }
    }
  }

  saving.value = true
  try {
    await systemApi.updateBackupSettings(settings.value)
    notification.success('备份默认策略已保存', '后续调度会按新默认策略判断')
  } catch (error: any) {
    notification.error('保存备份默认策略失败', error?.response?.data?.message || '')
  } finally {
    saving.value = false
  }
}

onMounted(() => {
  loadSettings()
})
</script>

<style scoped>
.backup-settings {
  background-color: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 20px;
}

.backup-settings h2 {
  margin: 0 0 16px 0;
  color: var(--text-primary);
  font-size: 18px;
}

.form {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.oss-form {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
  background-color: var(--bg-hover, rgba(128, 128, 128, 0.06));
  border-radius: 8px;
  border: 1px dashed var(--border-color);
}

.field-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.field-row .label {
  white-space: nowrap;
}

.hint {
  font-size: 12px;
  color: var(--text-secondary);
}

.hint code {
  font-family: "JetBrains Mono", monospace;
  background-color: var(--bg-hover, rgba(128, 128, 128, 0.12));
  padding: 1px 4px;
  border-radius: 3px;
}

.info-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.info-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 4px 0;
}

.label {
  color: var(--text-secondary);
  font-weight: 500;
}

.value {
  color: var(--text-primary);
  font-family: "JetBrains Mono", monospace;
}
</style>

<template>
  <n-modal
    v-model:show="show"
    title="上传存档"
    preset="dialog"
    :on-after-leave="handleCancel"
  >
    <n-upload
      ref="uploadRef"
      action="/api/saves/upload"
      :headers="uploadHeaders"
      :max="1"
      :auto-upload="false"
      accept=".wld,.wld.bak,.bak"
      :on-finish="handleFinish"
      :on-error="handleError"
    >
      <n-upload-dragger>
        <div class="upload-dragger-content">
          <p class="upload-icon">📁</p>
          <p class="upload-text">点击或拖拽存档文件到此处</p>
          <p class="upload-hint">支持 .wld、.wld.bak 和 .bak 存档文件</p>
        </div>
      </n-upload-dragger>
    </n-upload>

    <template #action>
      <n-button @click="handleCancel">取消</n-button>
      <n-button type="primary" :loading="loading" @click="handleUpload">
        上传
      </n-button>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { NModal, NUpload, NUploadDragger, NButton } from 'naive-ui'
import { useAuthStore } from '../../stores/auth'
import { useNotification } from '../../composables/useNotification'

interface Props {
  show: boolean
}

interface Emits {
  (e: 'update:show', value: boolean): void
  (e: 'uploaded'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

const authStore = useAuthStore()
const notification = useNotification()

const uploadRef = ref()
const loading = ref(false)

const show = computed({
  get: () => props.show,
  set: (value) => emit('update:show', value)
})

const uploadHeaders = computed(() => ({
  Authorization: `Bearer ${authStore.token}`
}))

function handleUpload() {
  loading.value = true
  uploadRef.value?.submit()
}

function handleFinish() {
  loading.value = false
  notification.success('存档已上传', '')
  emit('uploaded')
  handleCancel()
}

function handleError() {
  loading.value = false
  notification.error('上传失败', '')
}

function handleCancel() {
  show.value = false
  uploadRef.value?.clearFiles()
}
</script>

<style scoped>
:deep(.n-upload) {
  width: 100%;
}

.upload-dragger-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 20px 0;
}

.upload-icon {
  font-size: 36px;
  margin: 0;
}

.upload-text {
  color: var(--text-primary);
  font-size: 14px;
  margin: 0;
}

.upload-hint {
  color: var(--text-muted);
  font-size: 12px;
  margin: 0;
}
</style>

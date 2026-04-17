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
      list-type="text"
      accept=".wld"
      @finish="handleFinish"
      @error="handleError"
    />

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
import { NModal, NUpload, NButton } from 'naive-ui'
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
</style>

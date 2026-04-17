<template>
  <div class="save-manager">
    <div class="header">
      <h1>存档管理</h1>
      <n-button type="primary" @click="showUploadModal = true">
        + 上传存档
      </n-button>
    </div>

    <n-spin :show="loading">
      <div v-if="saves.length === 0" class="empty">
        <p>暂无存档</p>
      </div>
      <div v-else class="saves-grid">
        <save-card
          v-for="save in saves"
          :key="save.id"
          :save="save"
          server-id=""
          @import="() => handleImportSave(save.id, '')"
          @delete="() => handleDeleteSave(save.id)"
          @download="() => handleDownloadSave(save)"
        />
      </div>
    </n-spin>

    <save-upload-modal
      v-model:show="showUploadModal"
      @uploaded="handleSaveUploaded"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { NButton, NSpin } from 'naive-ui'
import { savesApi } from '../api/saves'
import { useNotification } from '../composables/useNotification'
import SaveCard from '../components/save/SaveCard.vue'
import SaveUploadModal from '../components/save/SaveUploadModal.vue'

const notification = useNotification()

const loading = ref(false)
const showUploadModal = ref(false)
const saves = ref<any[]>([])

async function loadSaves() {
  loading.value = true
  try {
    const response = await savesApi.getList()
    saves.value = response.data
  } catch (error) {
    notification.error('加载存档失败', '')
  } finally {
    loading.value = false
  }
}

async function handleImportSave(saveId: string, serverId: string) {
  try {
    await savesApi.importToServer(saveId, serverId)
    notification.success('存档已导入', '')
  } catch (error: any) {
    notification.error('导入失败', error?.response?.data?.message || '')
  }
}

async function handleDeleteSave(saveId: string) {
  try {
    await savesApi.delete(saveId)
    notification.success('存档已删除', '')
    loadSaves()
  } catch (error: any) {
    notification.error('删除失败', error?.response?.data?.message || '')
  }
}

async function handleDownloadSave(save: { id: string; name: string }) {
  try {
    const response = await savesApi.download(save.id)
    const url = window.URL.createObjectURL(response.data)
    const link = document.createElement('a')
    link.href = url
    link.download = save.name
    link.click()
    window.URL.revokeObjectURL(url)
    notification.success('下载开始', '')
  } catch (error: any) {
    notification.error('下载失败', error?.response?.data?.message || '')
  }
}

function handleSaveUploaded() {
  showUploadModal.value = false
  loadSaves()
}

onMounted(() => {
  loadSaves()
})
</script>

<style scoped>
.save-manager {
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

.empty {
  background-color: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 40px;
  text-align: center;
  color: var(--text-secondary);
  transition: background-color 0.3s, border-color 0.3s;
}

.saves-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
  gap: 16px;
}
</style>

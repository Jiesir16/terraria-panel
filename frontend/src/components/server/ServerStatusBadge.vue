<template>
  <n-tag :type="tagType" :bordered="false">
    {{ statusText }}
  </n-tag>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NTag } from 'naive-ui'

interface Props {
  status: 'stopped' | 'starting' | 'running' | 'stopping' | 'error'
}

const props = defineProps<Props>()

const statusMap: Record<string, string> = {
  stopped: '已停止',
  starting: '启动中',
  running: '运行中',
  stopping: '停止中',
  error: '错误'
}

const typeMap: Record<string, 'success' | 'info' | 'warning' | 'error'> = {
  stopped: 'info',
  starting: 'warning',
  running: 'success',
  stopping: 'warning',
  error: 'error'
}

const statusText = computed(() => statusMap[props.status] || '未知')
const tagType = computed(() => typeMap[props.status] || 'info')
</script>

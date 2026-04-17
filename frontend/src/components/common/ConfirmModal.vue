<template>
  <n-modal
    v-model:show="show"
    type="warning"
    title="确认操作"
    preset="confirm"
    :content="message"
    positive-text="确认"
    negative-text="取消"
    @positive-click="handleConfirm"
    @negative-click="handleCancel"
  />
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NModal } from 'naive-ui'

interface Props {
  show: boolean
  message: string
  title?: string
}

interface Emits {
  (e: 'update:show', value: boolean): void
  (e: 'confirm'): void
  (e: 'cancel'): void
}

const props = withDefaults(defineProps<Props>(), {
  title: '确认操作'
})

const emit = defineEmits<Emits>()

const show = computed({
  get: () => props.show,
  set: (value) => emit('update:show', value)
})

function handleConfirm() {
  emit('confirm')
  show.value = false
}

function handleCancel() {
  emit('cancel')
  show.value = false
}
</script>

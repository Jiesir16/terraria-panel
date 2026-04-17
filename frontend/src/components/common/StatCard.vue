<template>
  <div class="stat-card" :style="{ borderTopColor: color }">
    <div class="stat-title">{{ title }}</div>
    <div class="stat-value" :style="{ color }">{{ value }}</div>
  </div>
</template>

<script setup lang="ts" generic="T extends string | number">
interface Props {
  title: string
  value: T
  color?: string
}

withDefaults(defineProps<Props>(), {
  color: '#50C878'
})
</script>

<style scoped>
.stat-card {
  background-color: var(--bg-card);
  border: 1px solid var(--border-color);
  border-top: 4px solid #50C878;
  border-radius: 12px;
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  transition: background-color 0.3s, border-color 0.3s;
}

.stat-title {
  color: var(--text-secondary);
  font-size: 14px;
}

.stat-value {
  font-size: 28px;
  font-weight: bold;
  color: #50C878;
}
</style>

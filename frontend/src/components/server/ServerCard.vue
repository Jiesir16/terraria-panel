<template>
  <div class="server-card" @click="$emit('click')">
    <div class="card-header">
      <h3>{{ server.name }}</h3>
      <server-status-badge :status="server.status" />
    </div>

    <div class="card-info">
      <div class="info-row">
        <span class="label">版本:</span>
        <span class="value">{{ server.tshock_version }}</span>
      </div>
      <div class="info-row">
        <span class="label">端口:</span>
        <span class="value">{{ server.port }}</span>
      </div>
      <div class="info-row">
        <span class="label">玩家:</span>
        <span class="value">{{ server.player_count }} / {{ server.max_players }}</span>
      </div>
      <div class="info-row" v-if="server.world_name">
        <span class="label">世界:</span>
        <span class="value">{{ server.world_name }}</span>
      </div>
    </div>

    <div v-if="canControl" class="card-actions">
      <n-button
        v-if="server.status !== 'stopped'"
        text
        type="error"
        size="small"
        @click.stop="$emit('stop')"
      >
        停止
      </n-button>
      <n-button
        v-else
        text
        type="success"
        size="small"
        @click.stop="$emit('start')"
      >
        启动
      </n-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { NButton } from 'naive-ui'
import ServerStatusBadge from './ServerStatusBadge.vue'

interface Props {
  server: any
  canControl?: boolean
}

interface Emits {
  (e: 'click'): void
  (e: 'start'): void
  (e: 'stop'): void
}

withDefaults(defineProps<Props>(), {
  canControl: true
})
defineEmits<Emits>()
</script>

<style scoped>
.server-card {
  background-color: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 16px;
  cursor: pointer;
  transition: all 0.3s ease;
}

.server-card:hover {
  border-color: var(--color-primary);
  box-shadow: 0 4px 12px rgba(80, 200, 120, 0.1);
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.card-header h3 {
  margin: 0;
  color: var(--text-primary);
  font-size: 16px;
}

.card-info {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 12px;
}

.info-row {
  display: flex;
  justify-content: space-between;
  font-size: 12px;
}

.label {
  color: var(--text-secondary);
}

.value {
  color: var(--text-primary);
  font-family: "JetBrains Mono", monospace;
}

.card-actions {
  display: flex;
  gap: 8px;
  justify-content: center;
}
</style>

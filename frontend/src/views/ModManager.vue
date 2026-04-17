<template>
  <div class="mod-manager">
    <div class="header">
      <h1>Mod 管理</h1>
      <p class="subtitle">全局 Mod 管理库</p>
    </div>

    <n-alert type="info" closable :title="`提示`">
      这是全局 Mod 管理，您可以在各个服务器实例中使用这些 Mod。要上传 Mod，请在服务器详情页面进行。
    </n-alert>

    <div class="content">
      <p style="color: var(--text-secondary);">
        Mod 管理功能将在服务器详情页面中以标签页的形式提供。
        请选择一个服务器进行 Mod 的上传、启用/禁用和删除操作。
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { NAlert } from 'naive-ui'
</script>

<style scoped>
.mod-manager {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.header h1 {
  margin: 0;
  color: var(--text-primary);
}

.subtitle {
  color: var(--text-secondary);
  margin: 4px 0 0 0;
  font-size: 14px;
}

.content {
  background-color: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 20px;
  transition: background-color 0.3s, border-color 0.3s;
}
</style>

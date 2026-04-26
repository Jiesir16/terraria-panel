<template>
  <div class="settings">
    <router-view v-slot="{ Component, route }">
      <component :is="Component" :key="route.fullPath" />
    </router-view>
  </div>
</template>

<script setup lang="ts">
</script>

<style scoped>
.settings {
  display: flex;
  flex-direction: column;
  gap: 20px;
}
</style>

<template>
  <div class="sidebar">
    <div class="sidebar-icon" :class="{ active: currentView === 'models' }" @click="$emit('navigate', 'models')" title="Models">⊞</div>
    <div class="sidebar-icon" :class="{ active: currentView === 'chat' }" @click="$emit('navigate', 'chat')" title="Chat">💬</div>
    <div class="sidebar-icon" :class="{ active: currentView === 'developer' }" @click="$emit('navigate', 'developer')" title="Developer">⚡</div>
    <div class="sidebar-spacer"></div>
    <div class="sidebar-icon" :class="{ active: currentView === 'settings' }" 
      @click="$emit('navigate', currentView === 'settings' ? previousView : 'settings')" 
      title="Settings">⚙</div>
  </div>
</template>

<script setup lang="ts">
defineProps<{ currentView: string, previousView: string }>()
defineEmits(['navigate'])
</script>

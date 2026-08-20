<template>
  <div class="sidebar">
    <div class="sidebar-icon" :class="{ active: currentView === 'models' }" @click="$emit('navigate', 'models')" :title="t('sidebar.models')">⊞</div>
    <div class="sidebar-icon" :class="{ active: currentView === 'chat' }" @click="$emit('navigate', 'chat')" :title="t('sidebar.chat')">💬</div>
    <div class="sidebar-icon" :class="{ active: currentView === 'developer' }" @click="$emit('navigate', 'developer')" :title="t('sidebar.developer')">⚡</div>
    <div class="sidebar-spacer"></div>
    <div class="sidebar-icon" :class="{ active: currentView === 'settings' }" 
      @click="$emit('navigate', currentView === 'settings' ? previousView : 'settings')" 
      :title="t('sidebar.settings')">⚙</div>
  </div>
</template>

<script setup lang="ts">
import { t } from '../i18n'
defineProps<{ currentView: string, previousView: string }>()
defineEmits(['navigate'])
</script>

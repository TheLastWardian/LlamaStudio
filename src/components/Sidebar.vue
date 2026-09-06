<template>
  <div class="sidebar">
    <div class="sidebar-icon icon-models" :class="{ active: currentView === 'models' }" @click="$emit('navigate', 'models')" :title="t('sidebar.models')">
      <Layers :size="20" />
    </div>
    <div class="sidebar-icon icon-chat" :class="{ active: currentView === 'chat' }" @click="$emit('navigate', 'chat')" :title="t('sidebar.chat')">
      <Bot :size="20" />
    </div>
    <div class="sidebar-icon icon-developer" :class="{ active: currentView === 'developer' }" @click="$emit('navigate', 'developer')" :title="t('sidebar.developer')">
      <Cpu :size="20" />
    </div>
    <div class="sidebar-icon icon-discover" :class="{ active: currentView === 'discover' }" @click="$emit('navigate', 'discover')" :title="t('sidebar.discover')">
      <Download :size="20" />
    </div>
    <div class="sidebar-spacer"></div>
    <div class="sidebar-icon icon-settings" :class="{ active: currentView === 'settings' }" 
      @click="$emit('navigate', currentView === 'settings' ? previousView : 'settings')" 
      :title="t('sidebar.settings')">
      <Settings :size="20" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { Bot, Cpu, Download, Layers, Settings } from '@lucide/vue'
import { t } from '../i18n'

defineProps<{ currentView: string, previousView: string }>()
defineEmits(['navigate'])
</script>

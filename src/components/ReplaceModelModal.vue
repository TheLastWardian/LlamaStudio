<template>
  <div class="modal-overlay modal-overlay-center" @click.self="$emit('close')">
    <div class="modal replace-modal">
      <div class="modal-header">
        <span>{{ t('replace.title') }}</span>
      </div>
      <div class="replace-row" v-for="entry in entries" :key="entry.port" @click="$emit('choose', entry.port)">
        <span class="replace-name">{{ entry.model.name }}</span>
        <span class="replace-port">:{{ entry.port }}</span>
        <span v-if="entry.port === appConfig.chatPort" class="replace-chat">💬</span>
      </div>
      <button class="btn-secondary" style="width:100%; margin-top:12px;" @click="$emit('close')">{{ t('replace.cancel') }}</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { loadedModels } from '../stores/selectedModel'
import { appConfig } from '../stores/config'
import { t } from '../i18n'

const props = defineProps<{ candidates?: number[] }>()
defineEmits<{ (e: 'close'): void; (e: 'choose', port: number): void }>()

// candidatos: todos los cargados, o solo el del puerto en conflicto (caso A manual)
const entries = computed(() =>
  Object.entries(loadedModels.value)
    .map(([p, m]) => ({ port: Number(p), model: m }))
    .filter(e => !props.candidates || props.candidates.includes(e.port))
)
</script>

<style scoped>
.modal-overlay-center {
  align-items: center;
}
.replace-modal {
  min-width: 340px;
  max-width: 480px;
}
.replace-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border: 1px solid #2a2a2a;
  border-radius: 6px;
  margin-bottom: 6px;
  cursor: pointer;
}
.replace-row:hover {
  background: #1e2430;
}
.replace-name {
  flex: 1;
  color: #ddd;
  font-size: 13px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.replace-port {
  color: #5a8af5;
  font-size: 12px;
}
.replace-chat {
  font-size: 12px;
}
</style>

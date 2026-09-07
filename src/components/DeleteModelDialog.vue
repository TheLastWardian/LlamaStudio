<template>
  <div class="modal-overlay" @click.self="$emit('close')">
    <div class="modal-safe" @click.stop>
      <div class="modal modal-delete">
        <div class="modal-header">
          <span style="color:#fff; font-size:13px; font-weight:600; flex:1;">{{ t('deleteDialog.title') }}</span>
          <button class="modal-close" @click="$emit('close')">{{ t('modal.close') }}</button>
        </div>

        <div class="modal-model-list">
          <div class="del-row">
            <input type="checkbox" checked disabled />
            <span class="modal-model-name del-name">{{ main.name }}</span>
            <span class="tag" style="background:#2a2a2a; color:#888;">GGUF</span>
            <div style="flex:1"></div>
            <span class="del-size">{{ fmtSize(main.sizeBytes) }}</span>
          </div>

          <template v-if="related.length > 0">
            <div class="modal-section-title">
              {{ t('deleteDialog.relatedTitle') }}
              <button class="del-select-all" @click="selectAll">{{ t('deleteDialog.selectAll') }}</button>
            </div>
            <div v-for="r in related" :key="r.path" class="del-row" @click="toggle(r.path)">
              <input type="checkbox" :checked="checked.has(r.path)" @click.stop="toggle(r.path)" />
              <span class="modal-model-name del-name">{{ r.name }}</span>
              <span class="tag" :class="r.kind === 'vision' ? 'del-tag-vision' : 'del-tag-draft'">{{ r.kind.toUpperCase() }}</span>
              <div style="flex:1"></div>
              <span class="del-size">{{ fmtSize(r.sizeBytes) }}</span>
            </div>
          </template>
        </div>

        <div class="del-notes">
          <div v-if="isLoaded" class="del-note del-note-danger">{{ t('deleteDialog.loadedWarning') }}</div>
          <div v-else-if="useTrash" class="del-note del-note-trash">{{ t('deleteDialog.trashNote') }}</div>
          <div v-else class="del-note del-note-danger">{{ t('deleteDialog.permanentNote') }}</div>
        </div>

        <div class="modal-config-footer">
          <button class="btn-secondary" @click="$emit('close')">{{ t('deleteDialog.cancel') }}</button>
          <button class="btn-delete" :disabled="isLoaded" @click="confirm">{{ t('deleteDialog.delete') }}</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { t } from '../i18n'

const props = defineProps<{
  main: { path: string, name: string, sizeBytes: number }
  related: { path: string, name: string, sizeBytes: number, kind: 'vision' | 'draft' }[]
  useTrash: boolean
  isLoaded: boolean
}>()

const emit = defineEmits<{
  close: []
  confirm: [paths: string[]]
}>()

const checked = ref<Set<string>>(new Set([props.main.path]))

function toggle(path: string) {
  if (checked.value.has(path)) checked.value.delete(path)
  else checked.value.add(path)
}

function selectAll() {
  checked.value = new Set([props.main.path, ...props.related.map(r => r.path)])
}

function confirm() {
  emit('confirm', [...checked.value])
}

function fmtSize(bytes: number): string {
  if (bytes >= 1024 * 1024 * 1024) return (bytes / 1024 / 1024 / 1024).toFixed(2) + ' GB'
  if (bytes >= 1024 * 1024) return (bytes / 1024 / 1024).toFixed(1) + ' MB'
  return (bytes / 1024).toFixed(0) + ' KB'
}
</script>

<style scoped>
.modal-delete {
  width: 560px;
}

.del-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-bottom: 1px solid #222;
  cursor: pointer;
}

.del-row:hover {
  background: #252525;
}

.del-row input[type="checkbox"] {
  cursor: pointer;
}

.del-name {
  max-width: 340px;
}

.del-size {
  color: #555;
  font-size: 11px;
}

.del-select-all {
  margin-left: 8px;
  background: none;
  border: none;
  color: #5a8af5;
  cursor: pointer;
  font-size: 11px;
  text-transform: none;
}

.del-select-all:hover {
  color: #8ab0ff;
}

.del-tag-vision {
  background: #1a2a1a;
  color: #4af54a;
}

.del-tag-draft {
  background: #2a2a1a;
  color: #f5d04a;
}

.del-notes {
  padding: 8px 12px;
  border-top: 1px solid #2a2a2a;
}

.del-note {
  font-size: 11px;
  line-height: 1.5;
}

.del-note-danger {
  color: #f55a5a;
}

.del-note-trash {
  color: #4af54a;
}

.btn-delete {
  background: #3a1a1a;
  color: #f55a5a;
  border: 1px solid #5a2a2a;
  border-radius: 4px;
  padding: 6px 24px;
  font-size: 12px;
  cursor: pointer;
}

.btn-delete:hover:not(:disabled) {
  background: #4a2020;
}

.btn-delete:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>

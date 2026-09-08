<template>
  <div class="chat-view">
    <div class="topbar">
      <template v-if="activeLoadedModel">
        <span style="color:#5a8af5;">✦</span>
        <span style="color:#fff; font-size:13px;">{{ modelDisplayNames[activeLoadedModel.path] || activeLoadedModel.name }}</span>
        <button class="btn-secondary" style="margin-left:4px;" @click="showModal = true">▾</button>
        <div style="flex:1"></div>
        <button class="btn-eject" @click="eject">⏏</button>
      </template>
      <template v-else>
        <div style="flex:1"></div>
        <button class="btn-load" style="width:auto; padding:5px 12px;" @click="showModal = true">+ {{ t('chat.loadModelBtn') }}</button>
        <div style="flex:1"></div>
      </template>
    </div>

    <div v-if="!activeLoadedModel" class="chat-empty">
      <p>{{ t('chat.noModel') }}</p>
    </div>
    <iframe
      v-else
      :src="chatUrl"
      class="chat-frame"
    />

    <LoadModelModal v-if="showModal" @close="showModal = false" />
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { activeLoadedModel, modelLoading, removeLoaded } from '../stores/selectedModel'
import { modelDisplayNames } from '../stores/groups'
import { appConfig } from '../stores/config'
import { invoke } from '@tauri-apps/api/core'
import { t } from '../i18n'
import LoadModelModal from '../components/LoadModelModal.vue'

const showModal = ref(false)

const chatUrl = computed(() => `http://127.0.0.1:${appConfig.value.chatPort}/`)

async function eject() {
  await invoke('stop_model', { port: appConfig.value.chatPort })
  removeLoaded(appConfig.value.chatPort)
  modelLoading.value = false
}
</script>

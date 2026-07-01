<template>
  <BaseModal :visible="visible" :title="$t('gateway.connectionTitle')" @close="$emit('close')">
    <div class="flex flex-col gap-1">
      <!-- 端点 -->
      <div class="form-group">
        <label class="label">{{ $t('gateway.endpoint') }}</label>
        <div class="flex items-center gap-2">
          <div class="input font-mono flex min-w-0 flex-1 items-center">
            <span class="truncate">{{ endpointUrl }}</span>
          </div>
          <button class="btn btn--icon btn--secondary shrink-0" v-tooltip="$t('gateway.copyEndpoint')" @click="copy(endpointUrl)">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"/></svg>
          </button>
        </div>
      </div>

      <!-- 监听端口 -->
      <div class="form-group">
        <label class="label">{{ $t('gateway.port') }}</label>
        <input
          v-model.number="store.config.port"
          type="number"
          class="input font-mono"
          disabled
        />
        <p class="text-meta mt-1">{{ $t('gateway.portHint') }}</p>
      </div>

      <!-- 全局 Key -->
      <div class="form-group">
        <label class="label">{{ $t('gateway.apiKey') }}</label>
        <div class="flex items-center gap-2">
          <input
            v-model.trim="store.config.apiKey"
            :type="showKey ? 'text' : 'password'"
            class="input font-mono flex-1"
            :placeholder="$t('gateway.apiKeyPlaceholder')"
          />
          <button class="btn btn--icon btn--secondary shrink-0" v-tooltip="showKey ? $t('gateway.hideKey') : $t('gateway.showKey')" @click="showKey = !showKey">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M12 4.5C7 4.5 2.73 7.61 1 12c1.73 4.39 6 7.5 11 7.5s9.27-3.11 11-7.5C21.27 7.61 17 4.5 12 4.5zM12 17a5 5 0 1 1 0-10 5 5 0 0 1 0 10zm0-8a3 3 0 1 0 0 6 3 3 0 0 0 0-6z"/></svg>
          </button>
          <button class="btn btn--icon btn--secondary shrink-0" v-tooltip="$t('gateway.copyKey')" @click="copy(store.config.apiKey)">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"/></svg>
          </button>
          <button class="btn btn--icon btn--secondary shrink-0" v-tooltip="$t('gateway.regenerate')" @click="regenerate">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M17.65 6.35C16.2 4.9 14.21 4 12 4c-4.42 0-7.99 3.58-7.99 8s3.57 8 7.99 8c3.73 0 6.84-2.55 7.73-6h-2.08c-.82 2.33-3.04 4-5.65 4-3.31 0-6-2.69-6-6s2.69-6 6-6c1.66 0 3.14.69 4.22 1.78L13 11h7V4l-2.35 2.35z"/></svg>
          </button>
        </div>
      </div>
    </div>

    <template #footer>
      <button class="btn btn--secondary" @click="$emit('close')">{{ $t('common.close') }}</button>
      <button class="btn btn--primary" @click="save">{{ $t('common.save') }}</button>
      <button
        v-if="!store.status.running"
        class="btn btn--primary"
        :disabled="store.isTogglingServer"
        @click="start"
      >{{ store.isTogglingServer ? $t('gateway.starting') : $t('gateway.start') }}</button>
      <button
        v-else
        class="btn btn--danger"
        :disabled="store.isTogglingServer"
        @click="stop"
      >{{ store.isTogglingServer ? $t('gateway.stopping') : $t('gateway.stop') }}</button>
    </template>
  </BaseModal>
</template>

<script setup>
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useGatewayStore } from '../../stores/gateway'
import BaseModal from '../common/BaseModal.vue'

defineProps({
  visible: { type: Boolean, default: false }
})
const emit = defineEmits(['close'])

const { t } = useI18n()
const store = useGatewayStore()
const showKey = ref(false)

const endpointUrl = computed(
  () => store.status.address || `http://127.0.0.1:${store.config.port}/gateway`
)

const copy = async (text) => {
  if (!text) return
  try {
    await navigator.clipboard.writeText(text)
    window.$notify?.success(t('common.copySuccess'))
  } catch (e) {
    console.warn('copy failed', e)
  }
}

const regenerate = () => {
  const bytes = new Uint8Array(24)
  crypto.getRandomValues(bytes)
  const hex = Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('')
  store.config.apiKey = `sk-atm-${hex}`
  showKey.value = true
}

const save = async () => {
  await store.saveConfig()
  window.$notify?.success(t('gateway.settingsSaved'))
  emit('close')
}

const start = async () => {
  await store.saveConfig()
  try {
    await store.startServer()
    window.$notify?.success(t('gateway.startSuccess'))
  } catch {
    window.$notify?.error(t('gateway.startFailed'))
  }
}

const stop = async () => {
  try {
    await store.stopServer()
    window.$notify?.success(t('gateway.stopSuccess'))
  } catch {
    window.$notify?.error(t('gateway.stopFailed'))
  }
}
</script>

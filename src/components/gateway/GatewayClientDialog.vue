<template>
  <BaseModal
    :visible="visible"
    :title="client === 'codex' ? $t('gateway.clientSwitch.codexTitle') : $t('gateway.clientSwitch.claudeTitle')"
    :close-on-overlay="!isLoading"
    :close-on-esc="!isLoading"
    modal-class="max-w-[460px]"
    @close="handleClose"
  >
    <div class="space-y-4">
      <!-- Codex 专用：模型提供方 -->
      <div v-if="client === 'codex'" class="form-group">
        <label class="label">{{ $t('gateway.clientSwitch.modelProvider') }}</label>
        <input v-model="form.modelProvider" type="text" class="input" :disabled="isLoading" />
      </div>

      <!-- Codex 专用：单一模型 -->
      <div v-if="client === 'codex'" class="form-group">
        <label class="label">{{ $t('gateway.clientSwitch.model') }}</label>
        <input
          v-model="form.model"
          type="text"
          list="gw-client-models"
          :placeholder="$t('gateway.clientSwitch.modelPlaceholder')"
          class="input"
          :disabled="isLoading"
        />
      </div>

      <!-- Claude 专用：Opus / Sonnet / Haiku 三档模型 -->
      <div v-else class="form-group">
        <label class="label">{{ $t('platform.claude.dialog.fields.defaultModels') }}</label>
        <div class="grid grid-cols-3 gap-2">
          <div>
            <label class="mb-1 block text-xs text-text-secondary">{{ $t('platform.claude.dialog.fields.opusModel') }}</label>
            <input v-model="form.opusModel" type="text" list="gw-client-models" class="input" placeholder="claude-4-5-opus" :disabled="isLoading" />
          </div>
          <div>
            <label class="mb-1 block text-xs text-text-secondary">{{ $t('platform.claude.dialog.fields.sonnetModel') }}</label>
            <input v-model="form.sonnetModel" type="text" list="gw-client-models" class="input" placeholder="claude-4-5-sonnet" :disabled="isLoading" />
          </div>
          <div>
            <label class="mb-1 block text-xs text-text-secondary">{{ $t('platform.claude.dialog.fields.haikuModel') }}</label>
            <input v-model="form.haikuModel" type="text" list="gw-client-models" class="input" placeholder="claude-4-5-haiku" :disabled="isLoading" />
          </div>
        </div>
      </div>

      <datalist id="gw-client-models">
        <option v-for="id in modelOptions" :key="id" :value="id" />
      </datalist>

      <!-- Codex 专用：推理强度（与网关固定走 responses 协议，无需选择） -->
      <div v-if="client === 'codex'" class="form-group">
        <label class="label">{{ $t('gateway.clientSwitch.reasoningEffort') }}</label>
        <FloatingDropdown placement="bottom-start">
          <template #trigger="{ isOpen }">
            <button type="button" class="input flex items-center justify-between text-left" :disabled="isLoading">
              <span>{{ form.reasoningEffort }}</span>
              <svg class="w-4 h-4 transition-transform" :class="{ 'rotate-180': isOpen }" viewBox="0 0 24 24" fill="currentColor"><path d="M7 10l5 5 5-5z"/></svg>
            </button>
          </template>
          <template #default="{ close }">
            <button v-for="opt in reasoningOptions" :key="opt" @click="form.reasoningEffort = opt; close()" class="dropdown-item">{{ opt }}</button>
          </template>
        </FloatingDropdown>
      </div>

      <div class="form-group">
        <label class="label">{{ $t('gateway.clientSwitch.baseUrl') }}</label>
        <input :value="resolvedBaseUrl" type="text" class="input bg-muted font-mono" readonly />
      </div>

      <div class="form-group mb-0">
        <label class="label">{{ $t('gateway.clientSwitch.apiKey') }}</label>
        <input :value="apiKey" type="password" class="input bg-muted font-mono" readonly />
      </div>

      <div v-if="error" class="flex items-center gap-2 rounded-lg border border-danger/30 bg-danger/10 p-3 text-[13px] text-danger">
        <svg class="h-4 w-4 shrink-0" viewBox="0 0 24 24" fill="currentColor"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-2h2v2zm0-4h-2V7h2v6z"/></svg>
        {{ error }}
      </div>
    </div>

    <template #footer>
      <button class="btn btn--secondary" :disabled="isLoading" @click="handleClose">{{ $t('common.cancel') }}</button>
      <button class="btn btn--primary" :disabled="!canSubmit || isLoading" @click="handleSwitch">
        <span class="relative inline-flex items-center justify-center">
          <span :style="{ visibility: isLoading ? 'hidden' : 'visible' }">{{ $t('gateway.clientSwitch.switch') }}</span>
          <span v-if="isLoading" class="btn-spinner absolute inset-0 m-auto" aria-hidden="true"></span>
        </span>
      </button>
    </template>
  </BaseModal>
</template>

<script setup>
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { useGatewayStore } from '../../stores/gateway'
import BaseModal from '../common/BaseModal.vue'
import FloatingDropdown from '../common/FloatingDropdown.vue'

const props = defineProps({
  visible: { type: Boolean, default: false },
  client: { type: String, required: true } // 'codex' | 'claude'
})
const emit = defineEmits(['close', 'switched'])

const { t } = useI18n()
const store = useGatewayStore()

const reasoningOptions = ['low', 'medium', 'high', 'xhigh']
const form = ref({
  modelProvider: 'gateway',
  model: '',
  reasoningEffort: 'medium',
  wireApi: 'responses',
  opusModel: '',
  sonnetModel: '',
  haikuModel: ''
})
const isLoading = ref(false)
const error = ref('')

const modelOptions = computed(() => store.allModelIds())
const apiKey = computed(() => store.config.apiKey || '')

// 网关基址：去掉尾部 /v1，Codex 需要 /v1，Claude（Anthropic）自带 /v1/messages
const gatewayBase = computed(() => {
  const addr = store.status.address || `http://127.0.0.1:${store.config.port || 8766}/gateway`
  return addr.replace(/\/v1\/?$/, '')
})
const resolvedBaseUrl = computed(() =>
  props.client === 'codex' ? `${gatewayBase.value}/v1` : gatewayBase.value
)

const canSubmit = computed(() => {
  if (!apiKey.value) return false
  if (props.client === 'codex') return Boolean(form.value.modelProvider.trim() && form.value.model.trim())
  return true
})

watch(
  () => props.visible,
  (v) => {
    if (v) {
      error.value = ''
      isLoading.value = false
    }
  }
)

const handleClose = () => {
  if (!isLoading.value) emit('close')
}

const handleSwitch = async () => {
  if (!apiKey.value) {
    error.value = t('gateway.clientSwitch.needKey')
    return
  }
  if (!canSubmit.value) return
  error.value = ''
  isLoading.value = true
  try {
    if (props.client === 'codex') {
      await invoke('codex_switch_account', {
        modelProvider: form.value.modelProvider.trim(),
        model: form.value.model.trim(),
        reasoningEffort: form.value.reasoningEffort,
        wireApi: form.value.wireApi,
        baseUrl: resolvedBaseUrl.value,
        apiKey: apiKey.value
      })
    } else {
      await invoke('claude_switch_gateway', {
        baseUrl: resolvedBaseUrl.value,
        apiKey: apiKey.value,
        opusModel: form.value.opusModel.trim(),
        sonnetModel: form.value.sonnetModel.trim(),
        haikuModel: form.value.haikuModel.trim()
      })
    }
    window.$notify?.success(t('gateway.clientSwitch.switchSuccess'))
    emit('switched')
    emit('close')
  } catch (err) {
    console.error('Gateway client switch error:', err)
    error.value = err?.message || err || t('gateway.clientSwitch.switchFailed')
  } finally {
    isLoading.value = false
  }
}
</script>

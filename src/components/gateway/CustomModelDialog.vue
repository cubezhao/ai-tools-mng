<template>
  <BaseModal
    :visible="visible"
    :title="isEdit ? $t('gateway.models.editModelTitle') : $t('gateway.models.addModelTitle')"
    modal-class="!max-w-[480px]"
    @close="$emit('close')"
  >
    <div class="flex flex-col gap-3">
      <div class="form-group">
        <label class="label">{{ $t('gateway.models.modelId') }}</label>
        <input
          v-model.trim="form.id"
          class="input font-mono"
          list="gw-custom-model-ids"
          :placeholder="$t('gateway.models.modelIdPlaceholder')"
        />
        <datalist id="gw-custom-model-ids">
          <option v-for="id in idOptions" :key="id" :value="id" />
        </datalist>
        <p v-if="idError" class="mt-1 text-[12px] text-danger">{{ idError }}</p>
        <p v-else class="text-meta mt-1">{{ $t('gateway.models.modelIdHint') }}</p>
      </div>

      <div class="form-group">
        <label class="label">{{ $t('gateway.models.developerGroup') }}</label>
        <input
          v-model.trim="form.developer"
          class="input"
          list="gw-custom-developers"
          :placeholder="$t('gateway.models.developerPlaceholder')"
        />
        <datalist id="gw-custom-developers">
          <option v-for="d in developerOptions" :key="d" :value="d" />
        </datalist>
        <p class="text-meta mt-1">{{ $t('gateway.models.developerHint') }}</p>
      </div>

      <div class="form-group">
        <label class="label">{{ $t('gateway.models.detailPricing') }} · {{ $t('gateway.models.priceUnit') }}</label>
        <div class="grid grid-cols-2 gap-2">
          <div>
            <span class="mb-0.5 block text-[11px] text-text-muted">{{ $t('gateway.models.priceInput') }}</span>
            <input v-model.trim="form.input" type="number" min="0" step="any" class="input input--sm font-mono" placeholder="0" />
          </div>
          <div>
            <span class="mb-0.5 block text-[11px] text-text-muted">{{ $t('gateway.models.priceOutput') }}</span>
            <input v-model.trim="form.output" type="number" min="0" step="any" class="input input--sm font-mono" placeholder="0" />
          </div>
          <div>
            <span class="mb-0.5 block text-[11px] text-text-muted">{{ $t('gateway.models.priceCacheRead') }}</span>
            <input v-model.trim="form.cacheRead" type="number" min="0" step="any" class="input input--sm font-mono" placeholder="0" />
          </div>
          <div>
            <span class="mb-0.5 block text-[11px] text-text-muted">{{ $t('gateway.models.priceCacheWrite') }}</span>
            <input v-model.trim="form.cacheWrite" type="number" min="0" step="any" class="input input--sm font-mono" placeholder="0" />
          </div>
        </div>
        <p class="text-meta mt-1">{{ $t('gateway.models.priceOptionalHint') }}</p>
      </div>
    </div>

    <template #footer>
      <button class="btn btn--secondary" @click="$emit('close')">{{ $t('common.cancel') }}</button>
      <button class="btn btn--primary" :disabled="!canSave" @click="submit">{{ $t('common.save') }}</button>
    </template>
  </BaseModal>
</template>

<script setup>
import { computed, reactive, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import BaseModal from '../common/BaseModal.vue'
import { useGatewayStore } from '../../stores/gateway'

const props = defineProps({
  visible: { type: Boolean, default: false },
  model: { type: Object, default: null }
})
const emit = defineEmits(['close', 'save'])

const { t } = useI18n()
const store = useGatewayStore()

const blank = () => ({ id: '', developer: '', input: '', output: '', cacheRead: '', cacheWrite: '' })
const form = reactive(blank())

const isEdit = computed(() => Boolean(props.model && props.model.id))

// 已有模型 ID（用于补全；新建时若与目录重名则提示将覆盖目录定价）
const idOptions = computed(() => store.allModelIds())
const developerOptions = computed(() => [...new Set(store.models.map((p) => p.name).filter(Boolean))])

const idError = computed(() => {
  if (!form.id) return ''
  const dup = store.customModels.some((m) => m.id === form.id)
  if (dup && (!isEdit.value || props.model.id !== form.id)) return t('gateway.models.modelIdExists')
  return ''
})

const canSave = computed(() => Boolean(form.id) && !idError.value)

const numOrNull = (v) => {
  if (v === '' || v === null || v === undefined) return null
  const n = Number(v)
  return Number.isFinite(n) && n >= 0 ? n : null
}

watch(
  () => props.visible,
  (open) => {
    if (!open) return
    Object.assign(form, blank())
    if (props.model) {
      form.id = props.model.id || ''
      form.developer = props.model.developer || ''
      const cost = props.model.cost || {}
      form.input = typeof cost.input === 'number' ? String(cost.input) : ''
      form.output = typeof cost.output === 'number' ? String(cost.output) : ''
      form.cacheRead = typeof cost.cache_read === 'number' ? String(cost.cache_read) : ''
      form.cacheWrite = typeof cost.cache_write === 'number' ? String(cost.cache_write) : ''
    }
  },
  { immediate: true }
)

const submit = () => {
  if (!canSave.value) return
  const payload = { id: form.id }
  if (form.developer) payload.developer = form.developer
  const cost = {}
  const input = numOrNull(form.input)
  const output = numOrNull(form.output)
  const cacheRead = numOrNull(form.cacheRead)
  const cacheWrite = numOrNull(form.cacheWrite)
  if (input !== null) cost.input = input
  if (output !== null) cost.output = output
  if (cacheRead !== null) cost.cache_read = cacheRead
  if (cacheWrite !== null) cost.cache_write = cacheWrite
  if (Object.keys(cost).length) payload.cost = cost
  emit('save', payload, isEdit.value ? props.model.id : null)
}
</script>

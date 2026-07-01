<template>
  <BaseModal
    :visible="visible"
    :title="$t('gateway.diagnostics.routePreview')"
    modal-class="!max-w-[860px]"
    @close="$emit('close')"
  >
    <div class="flex flex-col gap-4">
      <div class="grid grid-cols-1 gap-3 md:grid-cols-[280px_minmax(0,1fr)_132px]">
        <div class="form-group mb-0">
          <label class="label">{{ $t('gateway.diagnostics.inbound') }}</label>
          <div class="grid grid-cols-3 gap-1 rounded-lg border border-border bg-muted p-1">
            <button
              v-for="p in protocols"
              :key="p.key"
              type="button"
              :class="['rounded-md px-2 py-1.5 text-[12px] transition', preview.protocol === p.key ? 'bg-surface text-accent shadow-sm' : 'text-text-secondary hover:text-text']"
              @click="preview.protocol = p.key"
            >
              {{ p.short }}
            </button>
          </div>
        </div>

        <div class="form-group mb-0">
          <label class="label">{{ $t('gateway.diagnostics.model') }}</label>
          <input
            v-model.trim="preview.model"
            class="input font-mono"
            list="gw-route-preview-models"
            :placeholder="$t('gateway.diagnostics.modelPlaceholder')"
          />
          <datalist id="gw-route-preview-models">
            <option v-for="id in modelOptions" :key="id" :value="id" />
          </datalist>
        </div>

        <div class="form-group mb-0">
          <label class="label">{{ $t('gateway.diagnostics.stream') }}</label>
          <button
            type="button"
            role="switch"
            :aria-checked="preview.stream"
            class="input flex items-center justify-between gap-2 text-left"
            @click="preview.stream = !preview.stream"
          >
            <span>{{ preview.stream ? $t('gateway.usage.streamOn') : $t('gateway.usage.streamOff') }}</span>
            <span class="relative inline-flex h-5 w-9 rounded-full transition-colors" :class="preview.stream ? 'bg-accent' : 'bg-border'">
              <span class="h-3.5 w-3.5 translate-y-[3px] rounded-full bg-text-inverse shadow transition-transform" :class="preview.stream ? 'translate-x-[18px]' : 'translate-x-0.5'"></span>
            </span>
          </button>
        </div>
      </div>

      <div v-if="!preview.model" class="rounded-lg border border-dashed border-border bg-muted/40 px-4 py-6 text-center text-[13px] text-text-muted">
        {{ $t('gateway.diagnostics.enterModel') }}
      </div>
      <div v-else-if="!selectedRoute" class="rounded-lg border border-warning/30 bg-warning/10 px-4 py-4 text-[13px] text-warning">
        {{ $t('gateway.diagnostics.noRoute', { model: preview.model }) }}
      </div>
      <div v-else class="rounded-lg border border-border bg-muted/20 p-3">
        <div class="rounded-lg border border-border bg-surface p-4">
          <div class="mb-3 flex flex-wrap items-center justify-between gap-2">
            <div class="text-[12px] font-medium uppercase tracking-[0.5px] text-text-muted">{{ $t('gateway.diagnostics.protocolPath') }}</div>
            <span :class="['badge badge--sm', needsTransform ? 'badge--warning-tech' : 'badge--success-tech']">
              {{ needsTransform ? $t('gateway.diagnostics.willTransform') : $t('gateway.diagnostics.sameWire') }}
            </span>
          </div>
          <div class="flex flex-col gap-2 md:flex-row md:items-stretch">
            <template v-for="(node, idx) in protocolPath" :key="node.key">
              <div
                :class="[
                  'min-w-0 flex-1 rounded-lg border px-3 py-3',
                  node.muted ? 'border-border bg-muted/40' : 'border-accent/30 bg-accent/10'
                ]"
              >
                <div class="text-[11px] uppercase tracking-wide text-text-muted">{{ node.title }}</div>
                <div class="mt-1 truncate font-mono text-[13px] font-semibold text-text" v-tooltip="node.value">{{ node.value }}</div>
                <div class="mt-1 text-[11px] text-text-secondary">{{ node.note }}</div>
              </div>
              <div v-if="idx < protocolPath.length - 1" class="grid place-items-center text-text-muted md:w-8">
                <span class="hidden md:inline">→</span>
                <span class="md:hidden">↓</span>
              </div>
            </template>
          </div>
        </div>

        <div class="mt-3 grid grid-cols-1 gap-2 md:grid-cols-4">
          <div v-for="step in routeSteps" :key="step.title" class="rounded-lg border border-border bg-surface p-3">
            <div class="text-[11px] uppercase tracking-wide text-text-muted">{{ step.title }}</div>
            <div class="mt-1 truncate font-mono text-[13px] text-text" v-tooltip="step.value">{{ step.value }}</div>
            <div class="mt-1 text-[11px] text-text-secondary">{{ step.note }}</div>
          </div>
        </div>
        <div class="mt-3 flex flex-wrap items-center gap-2">
          <span class="badge badge--sm">{{ $t('gateway.diagnostics.priority', { value: channelPriority(selectedRoute) }) }}</span>
          <span v-if="candidateRoutes.length > 1" class="badge badge--sm text-text-muted">
            {{ $t('gateway.diagnostics.fallbacks', { count: candidateRoutes.length - 1 }) }}
          </span>
        </div>
      </div>
    </div>

    <template #footer>
      <button class="btn btn--primary" @click="$emit('close')">{{ $t('common.close') }}</button>
    </template>
  </BaseModal>
</template>

<script setup>
import { computed, reactive, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useGatewayStore } from '../../stores/gateway'
import BaseModal from '../common/BaseModal.vue'

const props = defineProps({
  visible: { type: Boolean, default: false }
})
defineEmits(['close'])

const { t } = useI18n()
const store = useGatewayStore()

const preview = reactive({
  protocol: 'anthropic',
  model: '',
  stream: true
})

const protocols = computed(() => [
  { key: 'chat', short: 'Chat', label: t('gateway.diagnostics.protocolChat'), wire: 'chat' },
  { key: 'responses', short: 'Responses', label: t('gateway.diagnostics.protocolResponses'), wire: 'responses' },
  { key: 'anthropic', short: 'Claude', label: t('gateway.diagnostics.protocolAnthropic'), wire: 'anthropic' }
])

const enabledChannels = computed(() => store.channels.filter((c) => c.enabled !== false))

// 模型条目兼容字符串与 { id, upstream } 别名映射，匹配键取别名 id
const entryId = (m) => (typeof m === 'string' ? m : m?.id || '')

const modelOptions = computed(() => {
  const ids = new Set(store.allModelIds())
  for (const c of store.channels) {
    for (const m of c.models || []) ids.add(entryId(m))
  }
  return [...ids].sort()
})

const channelWire = (c) => {
  if (c.kind === 'codex_oauth') return 'responses'
  if (c.kind === 'anthropic') return 'anthropic'
  return c.wire === 'responses' ? 'responses' : 'chat'
}

const inboundWire = computed(() => protocols.value.find((p) => p.key === preview.protocol)?.wire || 'chat')

const channelPriority = (c) => (typeof c.priority === 'number' ? c.priority : 100)
const candidateRoutes = computed(() => {
  if (!preview.model) return []
  return enabledChannels.value
    .filter((c) => (c.models || []).some((m) => entryId(m) === preview.model))
    .map((c, i) => ({ ...c, _i: i }))
    .sort((a, b) => channelPriority(a) - channelPriority(b) || a._i - b._i)
})
const selectedRoute = computed(() => candidateRoutes.value[0] || null)
const needsTransform = computed(() => selectedRoute.value && inboundWire.value !== channelWire(selectedRoute.value))
const upstreamWire = computed(() => selectedRoute.value ? channelWire(selectedRoute.value) : '')

// 选中渠道对该别名的上游真实模型 id（无映射则等于别名）
const upstreamModel = computed(() => {
  if (!selectedRoute.value) return preview.model
  const entry = (selectedRoute.value.models || []).find((m) => entryId(m) === preview.model)
  const up = typeof entry === 'string' ? '' : entry?.upstream || ''
  return up && up !== preview.model ? up : preview.model
})
const hasAlias = computed(() => upstreamModel.value !== preview.model)

const protocolPath = computed(() => {
  if (!selectedRoute.value) return []
  return [
    {
      key: 'inbound',
      title: t('gateway.diagnostics.pathInbound'),
      value: wireLabel(inboundWire.value),
      note: t('gateway.diagnostics.clientProtocol')
    },
    {
      key: 'gateway',
      title: t('gateway.diagnostics.pathCanonical'),
      value: needsTransform.value ? t('gateway.diagnostics.gatewayTransform') : t('gateway.diagnostics.gatewayForward'),
      note: needsTransform.value ? t('gateway.diagnostics.gatewayTransformNote') : t('gateway.diagnostics.gatewayForwardNote'),
      muted: !needsTransform.value
    },
    {
      key: 'upstream',
      title: t('gateway.diagnostics.pathUpstream'),
      value: wireLabel(upstreamWire.value),
      note: t('gateway.diagnostics.channelProtocol')
    }
  ]
})

const routeSteps = computed(() => {
  if (!selectedRoute.value) return []
  const route = selectedRoute.value
  const protocol = protocols.value.find((p) => p.key === preview.protocol)
  return [
    { title: t('gateway.diagnostics.stepClient'), value: protocol?.label || preview.protocol, note: preview.stream ? t('gateway.usage.streamOn') : t('gateway.usage.streamOff') },
    {
      title: t('gateway.diagnostics.stepModel'),
      value: hasAlias.value ? `${preview.model} → ${upstreamModel.value}` : preview.model,
      note: hasAlias.value ? t('gateway.diagnostics.aliasRewriteNote') : t('gateway.diagnostics.requestModel')
    },
    { title: t('gateway.diagnostics.stepChannel'), value: route.name || route.id, note: t(kindLabel(route.kind)) },
    { title: t('gateway.diagnostics.stepWire'), value: wireLabel(channelWire(route)), note: needsTransform.value ? t('gateway.diagnostics.transformNote') : t('gateway.diagnostics.sameWireNote') }
  ]
})

const kindLabel = (k) => ({
  codex_oauth: 'gateway.channels.kindCodex',
  openai_compat: 'gateway.channels.kindCompat',
  anthropic: 'gateway.channels.kindAnthropic'
}[k] || k)

const wireLabel = (wire) => ({
  chat: 'OpenAI Chat',
  responses: 'OpenAI Responses',
  anthropic: 'Anthropic Messages'
}[wire] || wire)

watch(
  () => props.visible,
  (open) => {
    if (!open) return
    if (!store.models.length) store.loadModels()
  }
)
</script>

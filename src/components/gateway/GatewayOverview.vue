<template>
  <div class="flex flex-col gap-4">
    <section class="overflow-hidden rounded-lg border border-border bg-surface">
      <div class="border-b border-border bg-muted/30 px-4 py-3">
        <h3 class="text-[14px] font-semibold text-text">{{ $t('gateway.diagnostics.health') }}</h3>
      </div>
      <div class="grid grid-cols-1 divide-y divide-border md:grid-cols-4 md:divide-x md:divide-y-0">
        <button
          v-for="item in healthItems"
          :key="item.key"
          type="button"
          :class="[
            'group flex items-center justify-between gap-3 px-4 py-3 text-left',
            item.action ? 'cursor-pointer transition-colors hover:bg-accent/5 active:bg-accent/10 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent/40' : 'cursor-default'
          ]"
          @click="item.action?.()"
        >
          <div class="min-w-0">
            <div class="flex items-center gap-1.5 text-[13px] font-medium text-text">
              <span :class="{ 'group-hover:text-accent': item.action }">{{ item.label }}</span>
              <span
                v-if="item.action"
                class="rounded border border-accent/20 bg-accent/10 px-1.5 py-0.5 text-[10px] text-accent transition-colors group-hover:border-accent/40 group-hover:bg-accent/15"
              >
                {{ item.actionLabel || $t('gateway.diagnostics.viewList') }}
              </span>
            </div>
            <div class="mt-0.5 truncate text-[12px] text-text-muted">{{ item.detail }}</div>
          </div>
          <span
            :class="[
              'grid h-7 w-7 shrink-0 place-items-center rounded-full text-[15px] font-bold',
              item.ok ? 'bg-success/15 text-success' : 'bg-danger/15 text-danger'
            ]"
          >
            {{ item.ok ? '✓' : '×' }}
          </span>
        </button>
      </div>
    </section>

    <!-- 用量脉搏：聚合概览 -->
    <div class="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-5">
      <!-- 总请求（右上角今日新增） -->
      <div class="rounded-lg border border-border bg-surface px-4 py-3">
        <div class="flex items-baseline justify-between gap-1">
          <span class="font-mono text-[22px] font-semibold leading-none text-text" v-tooltip="stats.total.toLocaleString()">{{ formatCompact(stats.total) }}</span>
          <span v-if="stats.todayRequests" class="text-[11px] text-success">+{{ formatCompact(stats.todayRequests) }}</span>
        </div>
        <div class="mt-1.5 text-[11px] uppercase tracking-[0.5px] text-text-muted">{{ $t('gateway.usage.totalRequests') }}</div>
      </div>
      <div class="rounded-lg border border-border bg-surface px-4 py-3">
        <div :class="['font-mono text-[22px] font-semibold leading-none', stats.successRate >= 100 ? 'text-success' : 'text-warning']">{{ stats.successRate.toFixed(1) }}%</div>
        <div class="mt-1.5 text-[11px] uppercase tracking-[0.5px] text-text-muted">{{ $t('gateway.usage.successRate') }}</div>
      </div>
      <!-- 总 tokens（右上角今日新增） -->
      <div class="rounded-lg border border-border bg-surface px-4 py-3">
        <div class="flex items-baseline justify-between gap-1">
          <span class="font-mono text-[22px] font-semibold leading-none text-text" v-tooltip="stats.totalTokens.toLocaleString()">{{ formatCompact(stats.totalTokens) }}</span>
          <span v-if="stats.todayTokens" class="text-[11px] text-success">+{{ formatCompact(stats.todayTokens) }}</span>
        </div>
        <div class="mt-1.5 text-[11px] uppercase tracking-[0.5px] text-text-muted">{{ $t('gateway.usage.totalTokens') }}</div>
      </div>
      <div class="rounded-lg border border-border bg-surface px-4 py-3">
        <div class="font-mono text-[22px] font-semibold leading-none text-text">{{ stats.hitRate == null ? '—' : stats.hitRate.toFixed(1) + '%' }}</div>
        <div class="mt-1.5 flex items-center gap-1 text-[11px] uppercase tracking-[0.5px] text-text-muted">
          {{ $t('gateway.overview.cacheHitRate') }}
          <span class="cursor-help text-text-muted/70" v-tooltip="$t('gateway.overview.cacheHitRateHint')">?</span>
        </div>
      </div>
      <div class="rounded-lg border border-border bg-surface px-4 py-3">
        <div class="flex items-baseline gap-1">
          <span class="font-mono text-[22px] font-semibold leading-none text-text">{{ formatCost(stats.totalCost) }}</span>
          <span v-if="stats.partial" class="text-[11px] text-warning" v-tooltip="$t('gateway.usage.costPartialHint')">{{ $t('gateway.usage.costPartial') }}</span>
        </div>
        <div class="mt-1.5 flex items-center gap-1 text-[11px] uppercase tracking-[0.5px] text-text-muted">
          {{ $t('gateway.usage.totalCost') }}
          <span class="cursor-help text-text-muted/70" v-tooltip="$t('gateway.usage.costHint')">?</span>
        </div>
      </div>
    </div>

    <!-- 月度趋势图（近30天，从用量记录派生） -->
    <CodexUsageChart :loading="store.isLoadingUsage" :chart-data="dailyStats" />

    <ConnectionDialog :visible="showConnection" @close="showConnection = false" />

    <BaseModal
      :visible="showRoutableModels"
      :title="$t('gateway.diagnostics.routableModelsStatus')"
      modal-class="!max-w-[720px]"
      @close="showRoutableModels = false"
    >
      <div v-if="!routableModelList.length" class="rounded-lg border border-dashed border-border bg-muted/30 px-4 py-8 text-center text-[13px] text-text-muted">
        {{ $t('gateway.diagnostics.noRoutableModels') }}
      </div>
      <div v-else class="max-h-[56vh] overflow-y-auto rounded-lg border border-border">
        <div
          v-for="model in routableModelList"
          :key="model.id"
          class="flex items-start justify-between gap-3 border-b border-border px-3 py-2.5 last:border-b-0"
        >
          <div class="min-w-0">
            <div class="break-all font-mono text-[12px] font-medium text-text">{{ model.id }}</div>
            <div class="mt-1 flex flex-wrap gap-1">
              <span v-for="channel in model.channels" :key="channel.id" class="badge badge--sm">{{ channel.name }}</span>
            </div>
          </div>
          <span class="badge badge--sm text-text-muted">{{ $t('gateway.diagnostics.channelCount', { count: model.channels.length }) }}</span>
        </div>
      </div>
      <template #footer>
        <button class="btn btn--primary" @click="showRoutableModels = false">{{ $t('common.close') }}</button>
      </template>
    </BaseModal>
  </div>
</template>

<script setup>
import { computed, onActivated, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useGatewayStore } from '../../stores/gateway'
import { useGatewayPricing } from '../../composables/useGatewayPricing'
import BaseModal from '../common/BaseModal.vue'
import ConnectionDialog from './ConnectionDialog.vue'
import CodexUsageChart from '../openai/CodexUsageChart.vue'

const { t } = useI18n()
const store = useGatewayStore()
const { hasPrice, recordCost } = useGatewayPricing()
const showConnection = ref(false)
const showRoutableModels = ref(false)

const enabledChannels = computed(() => store.channels.filter((c) => c.enabled !== false))
// 模型条目兼容字符串与 { id, upstream } 别名映射，可路由键取别名 id
const entryId = (m) => (typeof m === 'string' ? m : m?.id || '')
const routableModels = computed(() => new Set(enabledChannels.value.flatMap((c) => Array.isArray(c.models) ? c.models.map(entryId) : [])))
const routableModelList = computed(() => {
  const map = new Map()
  for (const c of enabledChannels.value) {
    for (const m of c.models || []) {
      const id = entryId(m)
      if (!id) continue
      if (!map.has(id)) map.set(id, { id, channels: [] })
      map.get(id).channels.push({ id: c.id, name: c.name || c.id })
    }
  }
  return [...map.values()].sort((a, b) => a.id.localeCompare(b.id))
})

const healthItems = computed(() => [
  {
    key: 'server',
    label: t('gateway.diagnostics.serverStarted'),
    detail: store.status.running
      ? (store.status.address || `http://127.0.0.1:${store.config.port || 8766}/gateway`)
      : t('gateway.stopped'),
    ok: store.status.running,
    actionLabel: t('gateway.openConnection'),
    action: () => { showConnection.value = true }
  },
  {
    key: 'key',
    label: t('gateway.diagnostics.apiKeyStatus'),
    detail: store.config.apiKey ? t('gateway.diagnostics.configured') : t('gateway.diagnostics.missing'),
    ok: Boolean(store.config.apiKey)
  },
  {
    key: 'channels',
    label: t('gateway.diagnostics.channelsStatus'),
    detail: t('gateway.diagnostics.enabledChannelsCount', { enabled: enabledChannels.value.length, total: store.channels.length }),
    ok: enabledChannels.value.length > 0
  },
  {
    key: 'models',
    label: t('gateway.diagnostics.routableModelsStatus'),
    detail: t('gateway.diagnostics.routableModelsCount', { count: routableModels.value.size }),
    ok: routableModels.value.size > 0,
    actionLabel: t('gateway.diagnostics.viewList'),
    action: () => { showRoutableModels.value = true }
  }
])

// 用量脉搏：基于当前已加载记录的派生聚合
const stats = computed(() => {
  const list = store.usage
  const total = list.length
  const errors = list.filter((u) => u.status === 'error').length
  const totalTokens = list.reduce((sum, u) => sum + (u.promptTokens || 0) + (u.completionTokens || 0), 0)
  const totalCost = list.reduce((sum, u) => sum + recordCost(u), 0)
  const partial = list.some((u) => u.status !== 'error' && !hasPrice(u.model))
  const prompt = list.reduce((sum, u) => sum + (u.promptTokens || 0), 0)
  const cached = list.reduce((sum, u) => sum + (u.cachedTokens || 0), 0)
  const dayStart = new Date()
  dayStart.setHours(0, 0, 0, 0)
  const startTs = dayStart.getTime()
  const todayList = list.filter((u) => (u.createdAt || 0) >= startTs)
  const todayTokens = todayList.reduce((sum, u) => sum + (u.promptTokens || 0) + (u.completionTokens || 0), 0)
  return {
    total,
    successRate: total ? ((total - errors) / total) * 100 : 100,
    totalTokens,
    totalCost,
    partial,
    hitRate: prompt ? (cached / prompt) * 100 : null,
    todayRequests: todayList.length,
    todayTokens
  }
})

// 月度趋势：按天聚合请求数与 token（供折线图）
const dailyStats = computed(() => {
  const byDate = {}
  for (const u of store.usage) {
    if (!u.createdAt) continue
    const d = new Date(u.createdAt)
    if (Number.isNaN(d.getTime())) continue
    const key = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
    if (!byDate[key]) byDate[key] = { date: key, requests: 0, tokens: 0 }
    byDate[key].requests += 1
    byDate[key].tokens += (u.promptTokens || 0) + (u.completionTokens || 0)
  }
  return Object.values(byDate).sort((a, b) => a.date.localeCompare(b.date))
})

const formatCost = (n) => {
  if (!n) return '$0'
  return n >= 1 ? `$${n.toFixed(2)}` : `$${n.toFixed(4)}`
}

// 紧凑单位（1.2K / 3.4M），小于 1000 原样展示
const compactFormatter = new Intl.NumberFormat('en', { notation: 'compact', maximumFractionDigits: 1 })
const formatCompact = (n) => {
  const v = Number(n) || 0
  return v < 1000 ? v.toLocaleString() : compactFormatter.format(v)
}

onActivated(() => {
  store.loadStatus()
  store.loadUsage()
  if (!store.models.length) store.loadModels()
})
</script>

<template>
  <div class="flex flex-col gap-3">
    <div class="flex items-center justify-between">
      <p class="text-[12px] text-text-muted">{{ $t('gateway.channels.subtitle') }}</p>
      <div class="flex items-center gap-2">
        <button class="btn btn--sm btn--secondary" @click="showRoutePreview = true">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M4 7h10.17a3 3 0 1 0 0-2H4v2zm15-1a1 1 0 1 1-2 0 1 1 0 0 1 2 0zM4 19h10.17a3 3 0 1 0 0-2H4v2zm15-1a1 1 0 1 1-2 0 1 1 0 0 1 2 0zM4 13h2.17a3 3 0 0 0 5.66 0H20v-2h-8.17a3 3 0 0 0-5.66 0H4v2zm5-2a1 1 0 1 1 0 2 1 1 0 0 1 0-2z" /></svg>
          {{ $t('gateway.diagnostics.routePreview') }}
        </button>
        <button class="btn btn--sm btn--primary" @click="openAdd">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z" /></svg>
          {{ $t('gateway.channels.add') }}
        </button>
      </div>
    </div>

    <div v-if="!store.channels.length" class="flex flex-col items-center justify-center py-12 text-text-secondary">
      <p class="text-sm">{{ $t('gateway.channels.empty') }}</p>
      <button class="btn btn--secondary btn--sm mt-3" @click="openAdd">{{ $t('gateway.channels.addFirst') }}</button>
    </div>

    <div v-else class="grid grid-cols-1 gap-3 xl:grid-cols-2">
      <section
        v-for="c in sortedChannels"
        :key="c.id"
        :class="['rounded-lg border border-border bg-surface p-4 transition-colors hover:border-accent/30', { 'opacity-55': !isEnabled(c) }]"
      >
        <div class="flex items-start justify-between gap-3">
          <div class="min-w-0">
            <div class="truncate text-[14px] font-semibold text-text" v-tooltip="c.name">{{ c.name }}</div>
            <div class="mt-1 flex flex-wrap items-center gap-1">
              <span :class="['badge badge--sm', kindClass(c.kind)]">{{ $t(kindLabel(c.kind)) }}</span>
              <span v-if="c.kind === 'openai_compat'" class="badge badge--sm text-text-muted">{{ wireLabel(c.wire) }}</span>
              <span class="badge badge--sm text-text-muted">{{ $t('gateway.channels.priority') }} {{ channelPriority(c) }}</span>
            </div>
          </div>
          <div class="flex shrink-0 items-center gap-1">
            <button
              type="button"
              role="switch"
              :aria-checked="isEnabled(c)"
              :title="isEnabled(c) ? $t('gateway.channels.disable') : $t('gateway.channels.enable')"
              class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full transition-colors duration-200 focus:outline-none"
              :class="isEnabled(c) ? 'bg-accent' : 'bg-border'"
              @click="toggleEnabled(c)"
            >
              <span class="pointer-events-none inline-block h-3.5 w-3.5 transform rounded-full bg-text-inverse shadow transition duration-200" :class="isEnabled(c) ? 'translate-x-[18px]' : 'translate-x-0.5'" />
            </button>
            <button class="btn btn--icon-sm btn--ghost" v-tooltip="$t('common.edit')" @click="openEdit(c)">
              <svg width="15" height="15" viewBox="0 0 24 24" fill="currentColor"><path d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04a1 1 0 0 0 0-1.41l-2.34-2.34a1 1 0 0 0-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z"/></svg>
            </button>
            <button class="btn btn--icon-sm btn--ghost text-danger" v-tooltip="$t('common.delete')" @click="confirmDelete(c)">
              <svg width="15" height="15" viewBox="0 0 24 24" fill="currentColor"><path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/></svg>
            </button>
          </div>
        </div>

        <div class="mt-3 grid grid-cols-1 gap-2 md:grid-cols-2">
          <div class="rounded-lg border border-border bg-muted/20 px-3 py-2">
            <div class="text-[11px] uppercase tracking-wide text-text-muted">{{ $t('gateway.channels.target') }}</div>
            <div class="mt-1 truncate font-mono text-[12px] text-text" v-tooltip="target(c)">{{ target(c) }}</div>
          </div>
          <div class="rounded-lg border border-border bg-muted/20 px-3 py-2">
            <div class="text-[11px] uppercase tracking-wide text-text-muted">{{ $t('gateway.channels.model') }}</div>
            <div class="mt-1 flex items-center gap-1">
              <span class="font-mono text-[12px] text-text">{{ $t('gateway.channels.modelCount', { count: modelList(c).length }) }}</span>
              <span v-if="!modelList(c).length" class="text-[12px] text-text-muted">—</span>
            </div>
          </div>
        </div>

        <div v-if="sortedModels(c).length" class="mt-3 flex flex-wrap gap-1.5">
          <span v-for="m in sortedModels(c).slice(0, 6)" :key="m.id" class="badge badge--sm truncate font-mono">{{ m.label }}</span>
          <span
            v-if="sortedModels(c).length > 6"
            class="badge badge--sm cursor-default text-text-muted"
            v-tooltip="sortedModels(c).slice(6).map((m) => m.label).join(', ')"
          >+{{ sortedModels(c).length - 6 }}</span>
        </div>

        <div class="mt-3 flex flex-wrap items-center justify-between gap-3 border-t border-border pt-3">
          <div v-if="statsMap[c.id]?.recent.length" class="min-w-0 flex-1">
            <div class="flex items-center gap-[3px]">
              <span
                v-for="(u, i) in statsMap[c.id].recent"
                :key="i"
                :class="['h-4 w-[5px] rounded-full transition-transform hover:scale-y-110', u.status === 'error' ? 'bg-danger' : 'bg-success']"
                v-tooltip="recentTip(u)"
              />
            </div>
            <div class="mt-1 flex flex-wrap items-center gap-2">
              <span :class="['font-mono text-[11px] font-medium', rateClass(statsMap[c.id].successRate)]">{{ statsMap[c.id].successRate.toFixed(0) }}%</span>
              <span class="text-[11px] text-text-muted">{{ $t('gateway.channels.successRate') }}</span>
              <span v-if="statsMap[c.id].lastError" class="truncate text-[11px] text-danger" v-tooltip="statsMap[c.id].lastError.error || ''">
                {{ $t('gateway.channels.lastError') }}
              </span>
            </div>
          </div>
          <span v-else class="text-[12px] text-text-muted">{{ $t('gateway.channels.noRequests') }}</span>
          <label class="flex items-center gap-1 text-[12px] text-text-muted">
            {{ $t('gateway.channels.priority') }}
            <input
              type="number"
              min="0"
              :value="channelPriority(c)"
              class="w-14 rounded-md border border-border bg-muted px-1 py-1 text-center font-mono text-[12px] text-text-secondary focus:border-accent focus:bg-surface focus:text-text focus:outline-none"
              @change="updatePriority(c, $event.target.value)"
            />
          </label>
        </div>
      </section>
    </div>

    <ChannelDialog
      :visible="showDialog"
      :channel="editing"
      :accounts="store.bindableAccounts"
      @close="showDialog = false"
      @save="onSave"
    />
    <RoutePreviewDialog :visible="showRoutePreview" @close="showRoutePreview = false" />
  </div>
</template>

<script setup>
import { computed, onActivated, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useGatewayStore } from '../../stores/gateway'
import ChannelDialog from './ChannelDialog.vue'
import RoutePreviewDialog from './RoutePreviewDialog.vue'

const { t } = useI18n()
const store = useGatewayStore()

const showDialog = ref(false)
const showRoutePreview = ref(false)
const editing = ref(null)

// 按渠道聚合用量：近 10 次请求状态 + 整体成功率
const statsMap = computed(() => {
  const byChannel = {}
  for (const u of store.usage) {
    if (!u.channelId) continue
    ;(byChannel[u.channelId] ||= []).push(u)
  }
  const out = {}
  for (const c of store.channels) {
    const list = byChannel[c.id] || []
    const total = list.length
    const errors = list.filter((u) => u.status === 'error').length
    const recent = [...list]
      .sort((a, b) => new Date(b.createdAt) - new Date(a.createdAt))
      .slice(0, 10)
    const lastError = recent.find((u) => u.status === 'error') || null
    out[c.id] = { total, successRate: total ? ((total - errors) / total) * 100 : null, recent, lastError }
  }
  return out
})

const isEnabled = (c) => c.enabled !== false

const toggleEnabled = (c) => store.upsertChannel({ ...c, enabled: !isEnabled(c) })

const channelPriority = (c) => (typeof c.priority === 'number' ? c.priority : 100)

// 按优先级升序、同值保持原始顺序展示
const sortedChannels = computed(() =>
  store.channels
    .map((c, i) => ({ c, i }))
    .sort((a, b) => channelPriority(a.c) - channelPriority(b.c) || a.i - b.i)
    .map((x) => x.c)
)

const updatePriority = (c, val) => {
  const n = Number(val)
  const priority = Number.isFinite(n) && n >= 0 ? n : 100
  if (priority === channelPriority(c)) return
  store.upsertChannel({ ...c, priority })
}

const rateClass = (r) => (r >= 95 ? 'text-success' : r >= 80 ? 'text-warning' : 'text-danger')

const recentTip = (u) =>
  `${formatTime(u.createdAt)} · ${u.status === 'error' ? t('gateway.usage.statusError') : t('gateway.usage.statusOk')}`

const formatTime = (ts) => {
  if (!ts) return '—'
  const d = new Date(ts)
  return Number.isNaN(d.getTime()) ? String(ts) : d.toLocaleString()
}

const kindLabel = (k) => ({
  codex_oauth: 'gateway.channels.kindCodex',
  openai_compat: 'gateway.channels.kindCompat',
  anthropic: 'gateway.channels.kindAnthropic'
}[k] || k)

const kindClass = (k) => ({
  codex_oauth: 'badge--accent-tech',
  openai_compat: 'badge--success-tech',
  anthropic: 'badge--warning-tech'
}[k] || '')

const wireLabel = (wire) =>
  wire === 'responses' ? t('gateway.channels.apiTypeResponses') : t('gateway.channels.apiTypeChat')

const modelList = (c) => (Array.isArray(c.models) ? c.models : [])

// 模型条目兼容字符串与 { id, upstream } 别名映射
const entryId = (m) => (typeof m === 'string' ? m : m?.id || '')
const entryLabel = (m) => {
  if (typeof m === 'string') return m
  const id = m?.id || ''
  const up = m?.upstream || ''
  return up && up !== id ? `${id} → ${up}` : id
}

// 系统模型表：模型 ID → 发布时间，用于渠道模型按发布时间倒序
const releaseMap = computed(() => {
  const map = {}
  for (const p of store.models) {
    for (const m of p.models || []) {
      if (m?.id && m.release_date) map[m.id] = m.release_date
    }
  }
  return map
})

// 渠道支持的模型：能匹配系统模型表的按发布时间倒序，未匹配的保持原序排在后面
const sortedModels = (c) => {
  const list = modelList(c)
  return [...list]
    .sort((a, b) => {
      const ra = releaseMap.value[entryId(a)] || ''
      const rb = releaseMap.value[entryId(b)] || ''
      if (ra && rb) return rb.localeCompare(ra)
      if (ra) return -1
      if (rb) return 1
      return 0
    })
    .map((m) => ({ id: entryId(m), label: entryLabel(m) }))
}

const target = (c) => {
  if (c.kind === 'codex_oauth') {
    const acc = store.bindableAccounts.find((a) => a.id === c.accountId)
    return acc?.label || c.accountId || '—'
  }
  return c.baseUrl || t('gateway.channels.defaultBaseUrl')
}

const openAdd = () => {
  editing.value = null
  showDialog.value = true
}

const openEdit = (c) => {
  editing.value = { ...c }
  showDialog.value = true
}

const onSave = async (channel) => {
  await store.upsertChannel(channel)
  showDialog.value = false
  window.$notify?.success(t('gateway.channels.saved'))
}

const confirmDelete = async (c) => {
  const ok = await window.$confirm?.({
    title: t('gateway.channels.deleteTitle'),
    message: t('gateway.channels.deleteMessage', { name: c.name }),
    confirmText: t('common.delete'),
    variant: 'danger'
  })
  if (!ok) return
  await store.removeChannel(c.id)
  window.$notify?.success(t('gateway.channels.deleted'))
}

onMounted(() => {
  store.loadModels()
  if (!store.usage.length) store.loadUsage()
})
onActivated(() => store.loadUsage())
</script>

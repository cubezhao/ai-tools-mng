<template>
  <div class="flex h-full w-full flex-col overflow-hidden">
    <!-- 选项卡 -->
    <nav class="shrink-0 flex flex-wrap items-center justify-between gap-3 border-b border-border bg-surface px-6 py-2">
      <div class="flex items-center gap-1">
        <button
          v-for="tab in tabs"
          :key="tab.key"
          :class="['relative rounded-md px-3 py-2 text-[13px] font-medium transition-colors', activeTab === tab.key ? 'bg-accent/10 text-accent' : 'text-text-secondary hover:bg-muted hover:text-text active:bg-muted/70']"
          @click="activeTab = tab.key"
        >
          {{ $t(tab.label) }}
          <span v-if="activeTab === tab.key" class="absolute inset-x-3 -bottom-2 h-0.5 rounded bg-accent"></span>
        </button>
      </div>

      <div class="flex items-center gap-1 rounded-lg border border-border bg-muted/40 px-1.5 py-1">
        <span class="px-1.5 text-[12px] text-text-muted">{{ $t('gateway.clientSwitch.label') }}</span>
        <button class="btn btn--sm btn--ghost gap-1.5 px-2" v-tooltip="$t('gateway.clientSwitch.codexTooltip')" @click="openClient('codex')">
          <img src="/icons/openai.svg" alt="Codex" class="h-4 w-4 object-contain" />
          <span>Codex</span>
        </button>
        <button class="btn btn--sm btn--ghost gap-1.5 px-2" v-tooltip="$t('gateway.clientSwitch.claudeTooltip')" @click="openClient('claude')">
          <img src="/icons/claude.svg" alt="Claude Code" class="h-4 w-4 object-contain" />
          <span>Claude Code</span>
        </button>
      </div>
    </nav>

    <!-- 面板 -->
    <section class="flex-1 overflow-auto px-6 py-5">
      <Transition name="gw-tab" mode="out-in">
        <KeepAlive>
          <component :is="activeComponent" :key="activeTab" />
        </KeepAlive>
      </Transition>
    </section>

    <GatewayClientDialog :visible="showClientDialog" :client="clientType" @close="showClientDialog = false" />
  </div>
</template>

<script setup>
import { computed, onMounted, ref } from 'vue'
import { useGatewayStore } from '../../stores/gateway'
import GatewayOverview from '../gateway/GatewayOverview.vue'
import GatewayChannels from '../gateway/GatewayChannels.vue'
import GatewayModels from '../gateway/GatewayModels.vue'
import GatewayUsage from '../gateway/GatewayUsage.vue'
import GatewayClientDialog from '../gateway/GatewayClientDialog.vue'

const store = useGatewayStore()

const activeTab = ref('overview')
const showClientDialog = ref(false)
const clientType = ref('codex')

const openClient = (type) => {
  clientType.value = type
  showClientDialog.value = true
}

const tabs = [
  { key: 'overview', label: 'gateway.tabs.overview' },
  { key: 'channels', label: 'gateway.tabs.channels' },
  { key: 'models', label: 'gateway.tabs.models' },
  { key: 'usage', label: 'gateway.tabs.usage' }
]

const tabComponents = {
  overview: GatewayOverview,
  channels: GatewayChannels,
  models: GatewayModels,
  usage: GatewayUsage
}
const activeComponent = computed(() => tabComponents[activeTab.value])

onMounted(async () => {
  await store.loadConfig()
  await Promise.all([store.loadStatus(), store.loadBindableAccounts(), store.loadModels()])
})
</script>

<style scoped>
.gw-tab-enter-active,
.gw-tab-leave-active {
  transition: opacity 0.18s ease, transform 0.18s ease;
}

.gw-tab-enter-from {
  opacity: 0;
  transform: translateY(6px);
}

.gw-tab-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}

@media (prefers-reduced-motion: reduce) {
  .gw-tab-enter-active,
  .gw-tab-leave-active {
    transition: none;
  }

  .gw-tab-enter-from,
  .gw-tab-leave-to {
    transform: none;
  }
}
</style>

import { computed } from 'vue'
import { useGatewayStore } from '../stores/gateway'

// 网关用量计价：模型价格映射 + 单条费用估算（供总览与用量页共享）
export function useGatewayPricing() {
  const store = useGatewayStore()

  // 模型 ID → 价格（每百万 tokens），来自已同步的模型目录
  const priceMap = computed(() => {
    const map = {}
    for (const p of store.models) {
      for (const m of p.models || []) {
        if (m.cost && typeof m.cost === 'object') map[m.id] = m.cost
      }
    }
    return map
  })

  // 模型是否有可计价信息（含目录与自定义价格）
  const hasPrice = (model) => {
    const c = priceMap.value[model]
    return Boolean(c) && (typeof c.input === 'number' || typeof c.output === 'number')
  }

  // 单条记录的估算费用（无价格则返回 0）
  const recordCost = (u) => {
    const cost = priceMap.value[u.model]
    if (!cost) return 0
    const cached = u.cachedTokens || 0
    const cacheWrite = u.cacheWriteTokens || 0
    const prompt = u.promptTokens || 0
    const completion = u.completionTokens || 0
    const billableInput = Math.max(0, prompt - cached - cacheWrite)
    let c = 0
    if (typeof cost.input === 'number') c += (billableInput * cost.input) / 1e6
    if (typeof cost.cache_read === 'number') c += (cached * cost.cache_read) / 1e6
    else if (typeof cost.input === 'number') c += (cached * cost.input) / 1e6
    if (typeof cost.cache_write === 'number') c += (cacheWrite * cost.cache_write) / 1e6
    else if (typeof cost.input === 'number') c += (cacheWrite * cost.input) / 1e6
    if (typeof cost.output === 'number') c += (completion * cost.output) / 1e6
    return c
  }

  return { priceMap, hasPrice, recordCost }
}

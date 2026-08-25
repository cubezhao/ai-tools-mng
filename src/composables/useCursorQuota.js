import { computed } from 'vue'
import {
  getQuotaBarClass,
  grokBotRemainingPercent,
  planRemainingPercentFromCents,
  planSpendLabel,
  remainingPercentFromUsed
} from '../utils/cursorUsage'

export function useCursorQuota(getAccount) {
  const account = computed(() => getAccount())

  const plan = computed(() => account.value?.individual_usage?.plan || null)

  const planRemainingPercent = computed(() => planRemainingPercentFromCents(plan.value))

  const planSpend = computed(() => planSpendLabel(plan.value))

  const showPlanQuota = computed(() => planRemainingPercent.value !== null)

  const totalRemainingPercent = computed(() =>
    remainingPercentFromUsed(plan.value?.totalPercentUsed)
  )

  const autoRemainingPercent = computed(() =>
    remainingPercentFromUsed(plan.value?.autoPercentUsed)
  )

  const apiRemainingPercent = computed(() =>
    remainingPercentFromUsed(plan.value?.apiPercentUsed)
  )

  const grokBot = computed(() => {
    const usage = account.value?.individual_usage
    return usage?.grokBot || usage?.grok_bot || null
  })

  const grokBotRemaining = computed(() => grokBotRemainingPercent(grokBot.value))

  const showGrokBot = computed(() => grokBotRemaining.value !== null)

  const grokBotResetLabel = computed(() => {
    const end = grokBot.value?.periodEnd || grokBot.value?.period_end
    if (!end) return ''
    const date = new Date(end)
    if (Number.isNaN(date.getTime())) return ''
    return date.toLocaleDateString()
  })

  return {
    planRemainingPercent,
    planSpend,
    showPlanQuota,
    totalRemainingPercent,
    autoRemainingPercent,
    apiRemainingPercent,
    grokBotRemaining,
    showGrokBot,
    grokBotResetLabel,
    getQuotaBarClass
  }
}

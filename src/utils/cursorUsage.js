/** 将 usage-summary 写回账号，并保留上次已拉到的 Grok Bot 周额度 */
export function applyCursorUsageSummary(account, summary) {
  if (!account || !summary) return account

  if (summary.membershipType) {
    account.membership_type = summary.membershipType
  }

  const incoming = summary.individualUsage || {}
  const prev = account.individual_usage || {}
  const usage = { ...prev, ...incoming }
  if (summary.billingCycleStart) usage.billingCycleStart = summary.billingCycleStart
  if (summary.billingCycleEnd) usage.billingCycleEnd = summary.billingCycleEnd
  if (!incoming.grokBot && !incoming.grok_bot && (prev.grokBot || prev.grok_bot)) {
    usage.grokBot = prev.grokBot || prev.grok_bot
  }
  account.individual_usage = Object.keys(usage).length > 0 ? usage : account.individual_usage
  account.updated_at = Math.floor(Date.now() / 1000)
  return account
}

function toFiniteNumber(value) {
  if (value === null || value === undefined || value === '') return null
  const num = Number(value)
  return Number.isFinite(num) ? num : null
}

function clampPercent(percent) {
  return Math.min(100, Math.max(0, Math.round(percent)))
}

export function remainingPercentFromUsed(used) {
  const percentUsed = toFiniteNumber(used)
  if (percentUsed === null) return null
  return clampPercent(100 - percentUsed)
}

/**
 * 套餐（included usage）剩余百分比。
 *
 * 取 plan 的美分字段而非 autoPercentUsed / apiPercentUsed：官方说明这两个
 * percent 字段是另一套缓存聚合指标，不等于 used / limit，且在活跃账号上会滞后
 * 数天；仪表盘文案 “You've used X% of your included usage” 用的正是美分口径。
 */
export function planRemainingPercentFromCents(plan) {
  if (!plan) return null
  const limit = toFiniteNumber(plan.limit)
  if (limit === null || limit <= 0) return null

  const remaining = toFiniteNumber(plan.remaining)
  if (remaining !== null) {
    return clampPercent((remaining / limit) * 100)
  }
  const used = toFiniteNumber(plan.used)
  if (used === null) return null
  return clampPercent(100 - (used / limit) * 100)
}

/** 套餐额度的金额说明，如 `$12.88 / $20.00` */
export function planSpendLabel(plan) {
  if (!plan) return ''
  const limit = toFiniteNumber(plan.limit)
  if (limit === null || limit <= 0) return ''
  const remaining = toFiniteNumber(plan.remaining)
  const used = toFiniteNumber(plan.used) ?? (remaining === null ? null : limit - remaining)
  if (used === null) return ''
  return `$${(used / 100).toFixed(2)} / $${(limit / 100).toFixed(2)}`
}

export function grokBotRemainingPercent(grokBot) {
  if (!grokBot) return null
  const percentUsed = grokBot.percentUsed ?? grokBot.percent_used
  if (percentUsed !== null && percentUsed !== undefined) {
    return remainingPercentFromUsed(percentUsed)
  }
  const used = Number(grokBot.used)
  const limit = Number(grokBot.limit)
  if (Number.isFinite(used) && Number.isFinite(limit) && limit > 0) {
    return remainingPercentFromUsed((used / limit) * 100)
  }
  const remaining = Number(grokBot.remaining)
  if (Number.isFinite(remaining) && Number.isFinite(limit) && limit > 0) {
    return clampPercent((remaining / limit) * 100)
  }
  return null
}

export function getQuotaBarClass(percent) {
  if (percent === null || percent === undefined) return 'bg-text-muted'
  if (percent < 10) return 'bg-danger'
  if (percent < 30) return 'bg-warning'
  return 'bg-success'
}

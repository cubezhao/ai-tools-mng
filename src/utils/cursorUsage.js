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

/**
 * 把官方的「已用百分比」翻成剩余百分比。
 *
 * 用于 autoPercentUsed / apiPercentUsed / totalPercentUsed —— 它们分别是
 * Spending 页 Cursor Models / Other Models / Total 三条进度条，也是实际
 * 限流依据。totalPercentUsed 是两池花费除以两池合计预算，不是另外两个的均值。
 */
export function remainingPercentFromUsed(used) {
  const percentUsed = toFiniteNumber(used)
  if (percentUsed === null) return null
  return clampPercent(100 - percentUsed)
}

/**
 * 解析仪表盘横幅 “You've used X% of your included usage” 的分子分母。
 *
 * 官方说明该横幅用的是 includedSpend / limit，对应 usage-summary 里的
 * breakdown.included / limit，所以优先取 breakdown.included；它缺失时
 * 才退回 remaining、used（used 含 bonus 花费，会略高于横幅）。
 */
function planIncludedSpend(plan) {
  if (!plan) return null
  const limit = toFiniteNumber(plan.limit)
  if (limit === null || limit <= 0) return null

  const included = toFiniteNumber(plan.breakdown?.included)
  if (included !== null) return { spent: included, limit }

  const remaining = toFiniteNumber(plan.remaining)
  if (remaining !== null) return { spent: limit - remaining, limit }

  const used = toFiniteNumber(plan.used)
  if (used !== null) return { spent: used, limit }

  return null
}

/**
 * 套餐（included usage）剩余百分比，对齐仪表盘横幅文案。
 *
 * 这条口径只对应横幅那句话，不对应 Spending 页的进度条：进度条来自
 * autoPercentUsed / apiPercentUsed / totalPercentUsed，分母是两个独立
 * 用量池的预算，而 limit 是套餐月费美分（Pro = 2000），两者量级不同。
 */
export function planRemainingPercentFromCents(plan) {
  const spend = planIncludedSpend(plan)
  if (!spend) return null
  return clampPercent(100 - (spend.spent / spend.limit) * 100)
}

/** 套餐额度的金额说明，如 `$12.88 / $20.00` */
export function planSpendLabel(plan) {
  const spend = planIncludedSpend(plan)
  if (!spend) return ''
  return `$${(spend.spent / 100).toFixed(2)} / $${(spend.limit / 100).toFixed(2)}`
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

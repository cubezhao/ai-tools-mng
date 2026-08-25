import test from 'node:test'
import assert from 'node:assert/strict'

import {
  applyCursorUsageSummary,
  grokBotRemainingPercent,
  planRemainingPercentFromCents,
  planSpendLabel,
  remainingPercentFromUsed
} from './cursorUsage.js'

// 官方 usage-summary 真实响应片段：横幅“已用 64%”= includedSpend 1288 / limit 2000，
// 而三条池进度条（auto / api / total）分母是各池预算，停留在个位数。
const officialPlan = {
  enabled: true,
  used: 1288,
  limit: 2000,
  remaining: 712,
  breakdown: { included: 1288, bonus: 0, total: 1288 },
  autoPercentUsed: 3.8966666666666665,
  apiPercentUsed: 2.6444444444444444,
  totalPercentUsed: 3.733333333333334
}

test('套餐百分比对齐横幅口径，池百分比各自独立', () => {
  assert.equal(planRemainingPercentFromCents(officialPlan), 36)
  assert.equal(remainingPercentFromUsed(officialPlan.autoPercentUsed), 96)
  assert.equal(remainingPercentFromUsed(officialPlan.apiPercentUsed), 97)
  assert.equal(remainingPercentFromUsed(officialPlan.totalPercentUsed), 96)
})

test('横幅分子取 breakdown.included，而不是含 bonus 的 used', () => {
  const withBonus = {
    limit: 2000,
    used: 1500,
    remaining: 500,
    breakdown: { included: 1000, bonus: 500, total: 1500 }
  }
  assert.equal(planRemainingPercentFromCents(withBonus), 50)
  assert.equal(planSpendLabel(withBonus), '$10.00 / $20.00')
})

test('breakdown 缺失时依次退回 remaining、used', () => {
  assert.equal(planRemainingPercentFromCents({ limit: 2000, remaining: 500 }), 25)
  assert.equal(planRemainingPercentFromCents({ used: 1500, limit: 2000 }), 25)
  // included 为 null 不能当 0，否则会显示成额度全满
  assert.equal(
    planRemainingPercentFromCents({ limit: 2000, remaining: 500, breakdown: { included: null } }),
    25
  )
})

test('缺少额度信息时不给出百分比', () => {
  assert.equal(planRemainingPercentFromCents(null), null)
  assert.equal(planRemainingPercentFromCents({ used: 1500 }), null)
  assert.equal(planRemainingPercentFromCents({ used: 1500, limit: 0 }), null)
  assert.equal(planRemainingPercentFromCents({ used: null, limit: null }), null)
  assert.equal(planRemainingPercentFromCents({ limit: 2000 }), null)
})

test('超额时剩余归零，不会出现负值或超过 100', () => {
  assert.equal(planRemainingPercentFromCents({ used: 2600, limit: 2000 }), 0)
  assert.equal(remainingPercentFromUsed(137.5), 0)
  assert.equal(remainingPercentFromUsed(-5), 100)
})

test('percent 字段缺失时不当作 0 已用', () => {
  assert.equal(remainingPercentFromUsed(undefined), null)
  assert.equal(remainingPercentFromUsed(null), null)
  assert.equal(remainingPercentFromUsed('abc'), null)
  assert.equal(remainingPercentFromUsed(0), 100)
})

test('套餐金额说明沿用官方美分口径', () => {
  assert.equal(planSpendLabel(officialPlan), '$12.88 / $20.00')
  assert.equal(planSpendLabel({ limit: 2000, remaining: 500 }), '$15.00 / $20.00')
  assert.equal(planSpendLabel({ used: 1288 }), '')
})

// 2026-08-24 起 Cursor Models 池上限上调、Auto 改为按路由模型计费。
// 池上限变大只影响 autoPercentUsed / totalPercentUsed，limit（套餐月费美分）不变，
// 所以横幅口径和进度条口径会在同一账期内进一步拉开——这不是解析错误。
test('池上限中途上调时，横幅口径不受影响', () => {
  const before = { ...officialPlan }
  const afterLimitBump = {
    ...officialPlan,
    autoPercentUsed: 1.9483333333333333,
    totalPercentUsed: 1.8666666666666667
  }

  assert.equal(
    planRemainingPercentFromCents(afterLimitBump),
    planRemainingPercentFromCents(before)
  )
  assert.equal(remainingPercentFromUsed(afterLimitBump.autoPercentUsed), 98)
})

test('Grok Bot 周额度按剩余比例换算', () => {
  assert.equal(grokBotRemainingPercent({ percentUsed: 13.588743 }), 86)
  assert.equal(grokBotRemainingPercent({ used: 25, limit: 100 }), 75)
  assert.equal(grokBotRemainingPercent({ remaining: 40, limit: 100 }), 40)
  assert.equal(grokBotRemainingPercent(null), null)
})

test('写回用量摘要时保留上一次的 Grok Bot 周额度', () => {
  const account = {
    individual_usage: { grokBot: { percentUsed: 20 }, plan: { autoPercentUsed: 90 } }
  }
  applyCursorUsageSummary(account, {
    membershipType: 'ultra',
    billingCycleStart: '2026-08-02T14:11:55.000Z',
    individualUsage: { plan: officialPlan }
  })

  assert.equal(account.membership_type, 'ultra')
  assert.deepEqual(account.individual_usage.plan, officialPlan)
  assert.deepEqual(account.individual_usage.grokBot, { percentUsed: 20 })
  assert.equal(account.individual_usage.billingCycleStart, '2026-08-02T14:11:55.000Z')
})

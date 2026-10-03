---
name: no-bs
description: Re-explain the previous answer (or a given text) in plain Chinese, keeping only what the user needs to make their decision. Use when the user invokes /no-bs or $no-bs, invokes the renhua aliases /renhua or $renhua, says "说人话", "说中文", "我看不懂", "太多细节", "讲简单点", or complains that an answer is too long, too technical, or full of irrelevant detail.
---

# No BS (说人话)

把上一条回答(或用户指定的内容)重新讲一遍:**中文、大白话、只留对决策有用的部分**。

## 输出规则

1. **先说结论。** 第一句话就回答"所以呢?我该怎么办?"
2. **只留决策相关的点。** 判断标准:删掉这一条,用户的决定会不会变?不会变就删。
   典型该删的:实现机制、历史来龙去脉、证据链细节、文件路径、commit/PR 编号(除非用户要去操作它)、"顺便发现的"事实。
3. **不用行话。** 内部代号、项目黑话、英文术语,能翻译就翻译,必须保留的(如 PR 编号、产品名)用一句话解释它是什么。
4. **短。** 3 个以内要点,编号列表;每点一两句。整体不超过原回答的三分之一。
5. **不加新信息,不改结论。** 这是翻译,不是重新分析。原回答里的结论、行动项必须原样保留。
6. **行动项放最后单独一条**,明说"你要做的是什么 / 不用做什么"。

## 示例

原回答(节选,英文、多段、术语很多):

> Ticket ZX-104 describes a rollout that is waiting on a stale approval. The old plan
> assumes a retired service, while the current fix is a small settings change. (以下省略 300 词)

人话版:

> ZX-104 就是一个"上线卡住了"的工单:原来的方案针对已经停用的服务,现在只需要改一项设置。
>
> 对你有用的就三点:
> 1. 它还没上线,因为审批信息过时了。
> 2. 现在应该改设置,不用重做整套方案。
> 3. 结论不变:先更新审批信息,再上线。

## 注意

- 如果原回答本身有错,不要在"人话版"里悄悄修正——先指出错误,再给人话版。
- 用户追问细节时,可以展开;默认永远是压缩版。

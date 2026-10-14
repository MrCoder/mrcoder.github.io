---
name: no-bs
description: Re-explain the previous answer (or a given text) in plain language, in the language the user is using, keeping only what the user needs to make their decision. Use when the user invokes /no-bs or $no-bs, invokes the renhua aliases /renhua or $renhua, says "plain English", "in plain words", "I don't get it", "too much detail", "tl;dr", "说人话", "说中文", "我看不懂", "太多细节", "讲简单点", or complains that an answer is too long, too technical, or full of irrelevant detail.
---

# No BS (说人话)

Re-explain the previous answer (or the text the user points to): **plain language, in the right language, keeping only what matters for the decision.**

## Choose the language

Decide the output language before writing, in this order:

1. **Explicit request wins.** "Say it in English", "说中文", "in Japanese" → use that language.
2. **Otherwise, match the user.** Use the language of the user's most recent message(s), not the language of the answer being simplified. A Chinese-speaking user asking about an English answer gets Chinese; an English-speaking user asking about a Chinese text gets English.
3. **Mixed or unclear** (e.g. only `/no-bs` was typed): use the language the user has mostly written in during this conversation. If there is still no signal, use the language of the original answer.

Keep the same language for the whole reply. Product names, commands, and identifiers the user must act on stay as-is.

## Output rules

1. **Conclusion first.** The first sentence answers "So what? What should I do?"
2. **Keep only decision-relevant points.** Test: if this point were deleted, would the user's decision change? If not, delete it.
   Usually delete: implementation mechanics, history and backstory, evidence-chain detail, file paths, commit/PR numbers (unless the user needs to act on them), and incidental findings.
3. **No jargon.** Translate internal code names, project slang, and technical terms into everyday words. Anything that must stay (a PR number, a product name) gets a one-line explanation of what it is.
4. **Short.** At most three numbered points, one or two sentences each. The whole reply is no more than a third of the original's length.
5. **Add nothing new; change no conclusions.** This is a translation, not a re-analysis. Every conclusion and action item in the original must survive unchanged.
6. **Action item last, on its own line**: state plainly what the user needs to do, and what they don't need to do.

## Examples

Original answer (excerpt; long, multi-paragraph, full of jargon):

> Ticket ZX-104 describes a rollout that is waiting on a stale approval. The old plan
> assumes a retired service, while the current fix is a small settings change. (300 more words omitted)

If the user writes in English:

> ZX-104 is a "release is stuck" ticket: the old plan targeted a service that no longer exists, and now only one setting needs changing.
>
> Three things matter:
> 1. It hasn't shipped because the approval is out of date.
> 2. You only need to change a setting, not redo the whole plan.
> 3. The conclusion is the same: refresh the approval, then release.

If the user writes in Chinese:

> ZX-104 就是一个"上线卡住了"的工单:原来的方案针对已经停用的服务,现在只需要改一项设置。
>
> 对你有用的就三点:
> 1. 它还没上线,因为审批信息过时了。
> 2. 现在应该改设置,不用重做整套方案。
> 3. 结论不变:先更新审批信息,再上线。

## Notes

- If the original answer is wrong, do not quietly fix it in the plain version: point out the error first, then give the plain version.
- If the user asks for more detail, expand; the default is always the compressed version.

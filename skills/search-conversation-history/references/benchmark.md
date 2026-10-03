# Search-history benchmark mode

Use this reference only after the user explicitly invokes benchmark mode, for example:

```text
$search-conversation-history benchmark <question>
/search-conversation-history benchmark <question>
```

The benchmark compares a raw-source arm with the regular skill workflow for one frozen question. If a compatible index is unavailable, the skill arm uses the bundled extractor: label it **extractor fallback**, and do not describe that result as indexed acceleration. It is an experiment for deciding when the skill is useful, not a reason to benchmark every ordinary query. Do not repeat a heavy benchmark automatically for each question.

## Freeze the question

Use the same question body, unchanged, in both arms. This is a copyable Chinese example; it contains no expected answer, customer name, session ID, or other hardcoded historical result:

```text
请从本地 Codex、Claude Code 和 Cursor 历史中重建“在 API 规范编辑器中加入语法修复入口”这一决策的演变。按时间顺序给出：首次提出、关键变更及理由、最终状态；每条结论标出来源、原始时间戳、会话 ID，并说明哪些来源或时间段缺失、证据是否完整。只依据找到的原始用户/助手内容，区分原话和助手总结；不要猜测。
```

Useful question archetypes are:

- **Cross-tool evolving decision:** reconstruct one decision whose proposal, revision, and final state may be spread across Codex, Claude Code, and Cursor. Require original timestamps, sessions, reasons for changes, and completeness.
- **Unknown-session multi-query:** start without a session ID and answer several related questions about one topic (for example, first proposal, later objection, and follow-up action). Preserve provenance for each answer and report missing sources.
- **Known-file literal negative control:** given one known JSONL file or session and one exact term, ask for bounded literal matches. This should be handled with targeted `rg` or a small JSON parser; it is a control for the rule that simple searches do not use this skill, not evidence that a history index is faster.

Do not replace the frozen question with a paraphrase between arms. Do not seed either agent with a known answer, candidate IDs, parent conversation, quoted or replayed evidence, or subagent results.

## Isolate the two arms

Run the arms sequentially against the same history coverage, reading source stores without changing them. Freeze a source snapshot only if relevant records may change during the trial; do not copy all histories by default. Record which arm ran first and whether the operating-system, index, or model cache was warm; mark cache state unknown when unverified. Never let one arm's answer or hidden context guide the other.

| Arm | Allowed path | Required restriction |
|---|---|---|
| Raw | Original source stores, bounded `rg`/JSON parsing, and the same configured model/runtime | Must not load this skill, use the shared history index, call the index MCP, or run `extract_history.py`. |
| Indexed | The normal `search-conversation-history` workflow: refresh once, search visible indexed records, expand relevant records, and use Evidence fallback only when required | Must follow the skill's source filters, pagination, provenance, and visible-versus-raw rules. |

Use the same model, runtime, provider, authentication method, and billing channel for both arms. Disclose those settings in the result and make no silent fallback or provider change. Use `fork none` or an equivalent clean-agent setting: neither arm may inherit parent history or a known answer. If the runtime cannot provide clean agents, use two separate fresh-session prompts instead of pretending the current context is clean.

The question body remains identical; only the arm wrapper changes. Fresh-session wrappers can be:

```text
原始检索基线。只读取原始历史存储，并用有界的 rg 或 JSON 解析；禁止使用 search-conversation-history 技能、共享索引、索引 MCP 和 extract_history.py。不要猜测。

问题：<冻结问题>
```

```text
索引检索实验。按 search-conversation-history 技能的正常流程执行：先刷新一次索引，再查询可见索引，按需展开记录并保留来源、会话和时间戳；不要猜测。

问题：<冻结问题>
```

If clean agents cannot be created, these prompts must be sent in two independent fresh sessions. If token counters cannot be verified, record them as unavailable; never imitate a clean session, copy a parent's ledger, or estimate the missing values.

## Evidence and timing

Both arms must report enough evidence to compare answer quality:

- source coverage and unavailable stores;
- original timestamps and session IDs for material claims;
- whether the evidence is complete, partial, or contradictory;
- the distinction between direct user/assistant records and assistant synthesis.

Count only original user/assistant history as evidence. Mark quoted or replayed text and subagent output as disallowed evidence for the comparison; do not silently count it as an independent source.

Record two wall-clock measures:

1. **Evidence elapsed:** from the first search operation to evidence-complete output. Include the indexed refresh and every raw-arm correction needed to reach the required evidence quality.
2. **Active-agent wall time:** from the agent task start to its final result. If a parent waits for a child, record parent idle/wait separately instead of adding it to the search operation time.

Do not compare an index or SQL query's milliseconds with full agent wall time. A child agent's usage footer may be emitted before its final response and its latest ledger can lag; when possible, the parent should capture the child's ledger after the child finishes and label the boundary honestly. The ordinary measurement helper's scope is the first operation through the boundary before final response, with refresh included; final answer generation is excluded. Start it with a new private temporary directory and a nonexistent state path; do not pre-create or reuse the state file.

Use the measurement helper described in the main skill. For Codex, automatic discovery is valid only when `CODEX_THREAD_ID` and the first `session_meta` verify the current agent/session (`id`, plus `agent_path` when present). A benchmark child must pass its own verified log when auto-discovery could point to the parent. Other runtimes need an explicit verified log or an unavailable counter result. Report `uncached input`, `cached input`, and `output`; uncached input excludes cache reads and includes cache writes, so report a returned `cache_write` field separately, along with `tokens_through` when useful. Output already includes reasoning. Use request-boundary deltas; a whole-session ledger is valid only for a verified fresh agent devoted entirely to this trial. Do not estimate absent counters or report billing dollars.

## Interpret the comparison

Label the skill condition as **existing index**, **first build**, or **extractor fallback**. A first-build refresh includes index creation and is not interchangeable with a warm existing index. Record arm order and source/cache order, and call out a single trial; one run supports a descriptive observation, not a general speed claim.

Compute speedup only when the two arms reach equal evidence quality under the same completeness criteria. If one arm has weaker provenance, misses a source, uses disallowed evidence, or needs unresolved corrections, report the comparison as **incomplete** and show the quality difference instead of calculating a ratio.

Keep the parent aggregate metadata and result JSON outside the repository. Include the frozen-question hash, run timestamp, arm order, index state, source/cache order, model/runtime/provider/channel disclosure, clean-session status, evidence-quality verdict, wall measures, token availability, and limitations. Do not store raw history dumps in the repository.

End with a brief suitability judgement based on this question and measured quality/time/token results: name the scope or reconstruction work that justified the index, or recommend direct targeted search when its overhead dominates. A main agent using the same index can gain similar retrieval speed; this test compares workflows, not the skill label itself.

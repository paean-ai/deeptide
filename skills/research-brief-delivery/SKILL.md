# Research Brief Delivery

Use this skill when a general agent must produce a decision-ready research
brief, comparison, executive memo, or recommendation.

## Goal

Transform gathered information into a clear recommendation with evidence,
confidence, tradeoffs, and next actions. The output should be useful to a busy
decision maker, not merely a summary of sources.

## Brief Structure

Use this order:

1. Executive recommendation
2. Decision context
3. Options considered
4. Evidence matrix
5. Tradeoffs and risks
6. Open questions
7. Action plan
8. Sources

## Evidence Discipline

Separate:

- **Fact:** directly supported by a source or artifact.
- **Inference:** reasoned interpretation from facts.
- **Assumption:** needed to proceed but not verified.
- **Gap:** information still missing.

Label uncertainty explicitly.

## Option Comparison

A useful comparison includes:

- option name,
- user/business value,
- implementation or operating cost,
- risk,
- reversibility,
- recommendation note.

Example:

```md
| Option | Value | Cost | Risk | Note |
|---|---:|---:|---:|---|
| Private beta | High signal | Low | Low | Best first step |
| Public preview | High reach | Medium | Medium | Needs support docs |
| Full launch | Maximum reach | High | High | Premature today |
```

## Recommendation Quality Bar

A strong recommendation:

- Names one preferred option.
- Explains why alternatives are weaker.
- States confidence.
- Identifies the biggest risk.
- Gives concrete next steps.
- Does not overclaim beyond evidence.

## Source Hygiene

- Use primary sources when accuracy matters.
- Include links and dates for current information.
- Avoid long verbatim quotes.
- Mark stale or uncertain sources.
- Do not expose private data in public artifacts.

## Final Action Plan

Make actions executable:

- owner or role,
- next step,
- expected output,
- decision date,
- rollback or review trigger.

## Common Mistakes

- Writing a source summary instead of a decision memo.
- Hiding uncertainty.
- Treating all sources as equally reliable.
- Omitting the recommended option.
- Producing a beautiful report without a concrete next action.

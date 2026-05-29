#!/usr/bin/env python3
"""Benchmark: does cache-aligned memory layout actually win on DeepSeek?

Hypothesis under test
---------------------
DeepSeek's API caches request *prefixes* automatically (a cache hit is billed at
~10% of a miss). An agentic coding tool re-sends a large, mostly-stable prefix
(system prompt + tool schema + memory) every turn. The claim:

  * A **cache-aligned** layout — memory/system block byte-identical and
    positioned FIRST, with any volatile content (date, turn counter) pushed to
    the END of the request — keeps that prefix cached across turns.
  * A **naive** layout — a volatile line at the FRONT (e.g. "It is turn N,
    the time is ..."), or memory entries re-ordered each turn — busts the cache
    on every turn, because the first differing token cascades.

This script measures `prompt_cache_hit_tokens / prompt_tokens` per turn for both
layouts against the real API, so the design rule can be judged empirically
rather than asserted.

Usage
-----
    DEEPSEEK_API_KEY=sk-... python3 benchmarks/cache_memory_bench.py [--turns N]

Cost is negligible (max_tokens=8; we only read the usage field).
"""

import argparse
import json
import os
import secrets
import sys
import time
import urllib.request

API = "https://api.deepseek.com/chat/completions"
# Default to the real account model id (GET /models). `deepseek-chat` is a
# legacy alias that bills as flash, so it never exercises the pro tier — use
# `--model deepseek-v4-pro` for that.
MODEL = "deepseek-v4-flash"

# A per-run nonce mixed into the stable prefix. Without it, runs share DeepSeek's
# account-level content-keyed cache, so a second run hits the first run's warmed
# prefix and every layout looks ~equally cached — masking the real effect. A
# fresh salt each run forces a cold start, isolating the within-run layout
# difference we actually want to measure. Set in main().
RUN_SALT = ""


def post(key, body):
    req = urllib.request.Request(
        API,
        data=json.dumps(body).encode(),
        headers={
            "Authorization": f"Bearer {key}",
            "Content-Type": "application/json",
        },
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=60) as resp:
        return json.load(resp)


# --- synthetic-but-realistic prefix pieces -------------------------------

def base_system(seed_units=120):
    """Stand-in for the stable system prompt + tool-schema block (~few KB).

    The per-run RUN_SALT is constant within a run (so the prefix can still cache
    across turns) but unique across runs (so runs don't share cache)."""
    line = (
        "You are a senior coding agent operating in a terminal. Follow project "
        "conventions exactly, prefer existing patterns, and keep edits minimal. "
    )
    return (f"[build {RUN_SALT}] " + (line * seed_units)).strip()


def memory_entries():
    """A stable set of 'memory' facts — the thing we want cached."""
    return [
        "Build tool is pnpm, never npm or yarn.",
        "Tests run with `cargo test -p deeptide-core`.",
        "Default branch for code is dev, not main.",
        "The CN backend is Aliyun FC in cn-shanghai.",
        "Prefer PaeanTypography tokens over raw Font.system.",
        "Never re-add responsibility_spawnattrs_setdisclaim to Pty.swift.",
        "Mac app stores tokens in UserDefaults, not Keychain.",
        "Ring INTL routes to zero-api; CN routes to api-paeanone-com.",
        "Retention must use event-union activation anchoring, not lastActiveAt.",
        "Google Ads account PaeanAI is billed in CNY, not USD.",
    ]


def memory_block(entries):
    return "# project memory\n" + "\n".join(f"- {e}" for e in entries)


def volatile_line(turn):
    # The realistic cache-buster: a per-turn-changing line many tools put up top.
    return f"[session turn {turn} | clock 2026-05-29T{turn:02d}:00:00Z | ctx {turn*1731} tok]"


def conversation(turn):
    """Growing conversation tail — appended at the END for both layouts, so it
    is NOT the variable under test (it legitimately can't be cached as it grows;
    both layouts share it)."""
    msgs = []
    for t in range(1, turn + 1):
        msgs.append({"role": "user", "content": f"Step {t}: inspect module {t} and summarize."})
        msgs.append({"role": "assistant", "content": f"Module {t} reviewed; no issues found."})
    msgs.append({"role": "user", "content": "Reply with exactly: OK"})
    return msgs


def build_naive(turn, entries):
    """Volatile line FIRST → busts the whole downstream prefix every turn."""
    system = volatile_line(turn) + "\n" + base_system() + "\n" + memory_block(entries)
    return [{"role": "system", "content": system}] + conversation(turn)


def build_aligned(turn, entries):
    """Stable system+memory FIRST (byte-identical across turns); volatile content
    moved to the END as the last user message."""
    system = base_system() + "\n" + memory_block(entries)
    msgs = [{"role": "system", "content": system}] + conversation(turn)
    # volatile context appended at the very end — does not perturb the prefix
    msgs.append({"role": "user", "content": volatile_line(turn)})
    return msgs


def run_variant(key, name, builder, turns, entries_for_turn, model=MODEL):
    rows = []
    # Reasoning / pro tiers emit separate reasoning tokens before the answer, so
    # give them headroom; we still only read the usage/cache fields, never the
    # output.
    max_tokens = 64 if "pro" in model or "reasoner" in model else 8
    for turn in range(1, turns + 1):
        body = {
            "model": model,
            "max_tokens": max_tokens,
            "messages": builder(turn, entries_for_turn(turn)),
        }
        u = post(key, body)["usage"]
        prompt = u["prompt_tokens"]
        hit = u.get("prompt_cache_hit_tokens", 0)
        rows.append((turn, prompt, hit, hit / prompt if prompt else 0.0))
        time.sleep(1.0)  # let the server-side cache settle between turns
    return rows


def print_table(name, rows):
    print(f"\n=== {name} ===")
    print(f"{'turn':>4} {'prompt':>7} {'cache_hit':>9} {'hit_rate':>8}")
    for turn, prompt, hit, rate in rows:
        print(f"{turn:>4} {prompt:>7} {hit:>9} {rate:>7.1%}")
    # average hit rate over turns 2..N (turn 1 is always a cold miss)
    warm = rows[1:]
    avg = sum(r[3] for r in warm) / len(warm) if warm else 0.0
    print(f"warm (turn 2+) avg hit rate: {avg:.1%}")
    return avg


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--turns", type=int, default=5)
    ap.add_argument("--model", default=MODEL,
                    help="DeepSeek model id, e.g. deepseek-v4-flash (default) or deepseek-v4-pro")
    ap.add_argument("--salt", default=None,
                    help="run nonce (default: random) — keeps runs from sharing cache")
    args = ap.parse_args()

    key = os.environ.get("DEEPSEEK_API_KEY")
    if not key:
        print("DEEPSEEK_API_KEY not set", file=sys.stderr)
        sys.exit(1)

    global RUN_SALT
    RUN_SALT = args.salt or secrets.token_hex(8)
    print(f"model under test: {args.model}  |  run salt: {RUN_SALT}")
    stable_entries = memory_entries()

    # Naive layout A: volatile prefix line.
    naive_rows = run_variant(
        key, "NAIVE (volatile line first)", build_naive, args.turns,
        lambda _turn: stable_entries, model=args.model,
    )

    # Naive layout B: memory order shuffled each turn (another common cache-buster),
    # placed first but NOT byte-stable.
    def shuffled(turn):
        e = list(stable_entries)
        # deterministic rotation by turn → order changes every turn
        return e[turn % len(e):] + e[: turn % len(e)]

    naive_shuffle_rows = run_variant(
        key, "NAIVE (memory reordered each turn)",
        lambda t, e: [{"role": "system", "content": base_system() + "\n" + memory_block(e)}] + conversation(t),
        args.turns, shuffled, model=args.model,
    )

    # Aligned layout: stable system+memory first, volatile content last.
    aligned_rows = run_variant(
        key, "ALIGNED (stable prefix, volatile last)", build_aligned, args.turns,
        lambda _turn: stable_entries, model=args.model,
    )

    a = print_table("NAIVE — volatile line first", naive_rows)
    b = print_table("NAIVE — memory reordered each turn", naive_shuffle_rows)
    c = print_table("ALIGNED — stable prefix, volatile last", aligned_rows)

    print("\n================ VERDICT ================")
    print(f"naive (volatile-first)      warm hit rate: {a:.1%}")
    print(f"naive (reordered-memory)    warm hit rate: {b:.1%}")
    print(f"aligned (stable-prefix)     warm hit rate: {c:.1%}")
    if c > max(a, b) + 0.25:
        print("RESULT: cache-aligned layout is materially better — hypothesis CONFIRMED.")
    else:
        print("RESULT: no material difference — hypothesis NOT supported. Re-examine.")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Benchmark: automatic memory capture quality + real cost.

Auto-capture only earns its place if it (a) reliably extracts the durable
facts worth remembering, (b) does NOT capture one-off/ephemeral noise, and
(c) is cheap enough to run at the end of every session. This measures all
three against the live DeepSeek API.

It mirrors the extraction prompt in `src/memory_capture.rs` (keep them in
sync). For each synthetic session it plants K durable facts among M ephemeral
distractors, then scores:
  * recall      — fraction of planted durable facts captured
  * false-pos   — captured items that match an ephemeral distractor
and sums real token usage to report ¥ cost per session.

Usage:
    DEEPSEEK_API_KEY=sk-... python3 benchmarks/auto_capture_bench.py
"""

import json
import os
import sys
import urllib.request

API = "https://api.deepseek.com/chat/completions"
MODEL = "deepseek-v4-flash"  # auto-capture is a cheap background task

# deepseek-v4-flash published pricing (¥ per 1M tokens). Adjust if it changes;
# the token counts below are authoritative regardless.
PRICE_IN_PER_M = 2.0
PRICE_OUT_PER_M = 3.0


def post(key, body):
    req = urllib.request.Request(
        API,
        data=json.dumps(body).encode(),
        headers={"Authorization": f"Bearer {key}", "Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=120) as resp:
        return json.load(resp)


def capture_prompt(transcript, existing_titles):
    known = "\n".join(f"- {t}" for t in existing_titles) if existing_titles else "(none yet)"
    return (
        "You maintain a coding agent's long-term memory. Read the session "
        "transcript and extract only DURABLE, REUSABLE facts worth remembering for "
        "future sessions: stable user preferences, project conventions, environment "
        "constraints, and corrections the user made.\n\n"
        "Do NOT capture: one-off task details, transient state, file contents, "
        "anything already covered by existing memory, or anything you are unsure is "
        "durable.\n\n"
        f"Existing memory (do not duplicate these):\n{known}\n\n"
        "Return ONLY a JSON array, no prose. Each item:\n"
        '  {"text": "<concise fact>", "category": "user|feedback|project|reference", "confidence": 0.0-1.0}\n'
        "Return [] if nothing is worth keeping.\n\n"
        f"Transcript:\n{transcript}"
    )


# Each session: (transcript, [durable fact key-phrases], [ephemeral key-phrases]).
SESSIONS = [
    (
        "USER: Always use pnpm in this repo, never npm.\n"
        "ASSISTANT: Got it.\n"
        "USER: Also, run the formatter before every commit.\n"
        "ASSISTANT: Understood. Now fix the typo on line 42 of header.tsx.\n"
        "USER: yes, and the build is currently failing because of that.\n"
        "ASSISTANT: Fixed the typo and the build passes now.",
        ["pnpm", "formatter before"],            # durable
        ["line 42", "typo", "build is currently failing"],  # ephemeral
    ),
    (
        "USER: Our prod database is read-only from the app; writes go through the queue.\n"
        "ASSISTANT: Noted.\n"
        "USER: Restart the staging server, it's stuck.\n"
        "ASSISTANT: Restarted staging.\n"
        "USER: Remember the API base url is api.example.cn for the CN region.\n"
        "ASSISTANT: Saved.",
        ["read-only", "queue", "api.example.cn"],   # durable
        ["restart the staging server", "stuck"],     # ephemeral
    ),
    (
        "USER: I prefer concise answers, no preamble.\n"
        "ASSISTANT: Will do.\n"
        "USER: What's 2+2?\nASSISTANT: 4.\n"
        "USER: The deploy token expires today, rotate it after.\n"
        "ASSISTANT: I'll flag that.",
        ["concise", "no preamble"],                  # durable
        ["2+2", "deploy token expires today"],       # ephemeral (one-off)
    ),
]


def contains_any(haystack, phrases):
    h = haystack.lower()
    return any(p.lower() in h for p in phrases)


def main():
    key = os.environ.get("DEEPSEEK_API_KEY")
    if not key:
        print("DEEPSEEK_API_KEY not set", file=sys.stderr)
        sys.exit(1)

    print(f"model: {MODEL}\n")
    total_in = total_out = 0
    total_planted = total_recalled = total_fp = total_captured = 0

    for i, (transcript, durable, ephemeral) in enumerate(SESSIONS, 1):
        body = {
            "model": MODEL,
            "max_tokens": 512,
            "temperature": 0.0,
            "messages": [{"role": "user", "content": capture_prompt(transcript, [])}],
        }
        resp = post(key, body)
        u = resp["usage"]
        total_in += u["prompt_tokens"]
        total_out += u["completion_tokens"]
        content = resp["choices"][0]["message"]["content"]

        # Parse the JSON array out of the reply.
        try:
            start = content.index("[")
            end = content.rindex("]") + 1
            facts = json.loads(content[start:end])
        except (ValueError, json.JSONDecodeError):
            facts = []
        captured = [f.get("text", "") for f in facts if isinstance(f, dict)]

        # recall: each planted durable fact captured by some item?
        recalled = sum(1 for kp in durable if any(kp.lower() in c.lower() for c in captured))
        # false positives: captured items matching an ephemeral distractor.
        fp = sum(1 for c in captured if contains_any(c, ephemeral))

        total_planted += len(durable)
        total_recalled += recalled
        total_fp += fp
        total_captured += len(captured)

        print(f"session {i}: captured {len(captured)} | recall {recalled}/{len(durable)} | false-pos {fp}")
        for c in captured:
            print(f"    - {c}")

    recall = total_recalled / total_planted if total_planted else 0.0
    fp_rate = total_fp / total_captured if total_captured else 0.0
    cost = total_in / 1e6 * PRICE_IN_PER_M + total_out / 1e6 * PRICE_OUT_PER_M
    per_session = cost / len(SESSIONS)

    print("\n================ RESULT ================")
    print(f"durable-fact recall : {total_recalled}/{total_planted} = {recall:.0%}")
    print(f"false-positive rate : {total_fp}/{total_captured} = {fp_rate:.0%}")
    print(f"tokens              : in={total_in}  out={total_out}  ({len(SESSIONS)} sessions)")
    print(f"est. cost           : ¥{cost:.4f} total  =>  ¥{per_session:.4f}/session")
    print("(token counts are authoritative; ¥ uses listed flash pricing — confirm via dashboard delta)")
    verdict_ok = recall >= 0.8 and fp_rate <= 0.2 and per_session < 0.01
    print("VERDICT:", "GOOD — accurate and cheap enough to run every session"
          if verdict_ok else "NEEDS REVIEW — check recall / false-pos / cost above")


if __name__ == "__main__":
    main()

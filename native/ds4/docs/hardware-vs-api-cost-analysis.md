# Local ds4 Hardware vs DeepSeek V4 Flash API: Economic Comparison

This note compares **total cost of ownership (TCO)** for running DeepSeek V4 Flash locally via **ds4.c** on high-end Mac hardware (as described in the project README benchmarks) against **hosted DeepSeek V4 Flash** API pricing. It is an **order-of-magnitude economic model**, not tax or procurement advice; prices move with region, promotions, and resale markets.

**Primary API source (retrieved for this analysis):** [DeepSeek API — Models & Pricing](https://api-docs.deepseek.com/quick_start/pricing) (`deepseek-v4-flash`, USD per **1M tokens**):

| Charge component | USD / 1M tokens |
|------------------|-----------------|
| Input — cache miss | **0.14** |
| Input — cache hit | **0.0028** |
| Output | **0.28** |

DeepSeek applies **prefix caching** automatically for repeated long inputs; cache-hit pricing is far below cache-miss pricing. Output tokens are priced **2×** cache-miss input tokens in this table.

---

## 1. Hardware classes referenced by this project

The [`README.md`](../README.md) throughput table uses:

1. **MacBook Pro 16″, M3 Max, 128 GB unified memory** — primary “q2 / 128 GB RAM” class cited for running the 2-bit-oriented GGUF comfortably.
2. **Mac Studio, M3 Ultra, 512 GB unified memory** — higher throughput; **q4** numbers appear here (q4 is positioned as “≥ 256 GB RAM machines” for downloads).

These are **benchmark configurations**, not a guarantee that every Apple SKU or future chip matches the same dollars or watts.

---

## 2. Retail and secondary-market price estimates (May 2026)

Apple’s online storefront changes by generation (e.g. newer MacBook Pro generations appear over time). The numbers below mix **third-party retail/discount listings**, **press coverage of maxed configurations**, and **secondary-market anecdotes**. Treat wide bands as honest uncertainty.

### 2.1 MacBook Pro — M3 Max, 128 GB RAM

| Channel | Approximate USD range | Notes |
|--------|------------------------|-------|
| **New / authorized retail** | **~$4,300 – $5,500** | High-memory BTO SKUs are expensive; street/discount pricing often sits below list. Example retail listings in this band appeared around **$4.3k–$4.4k** for 128 GB / multi-TB configs during 2025–2026 discount cycles. |
| **Used (Swappa, eBay, etc.)** | **~$2,800 – $4,200** | Wide spread by SSD size, battery cycle count, warranty, and cosmetics. Maxed **M3 Max / 128 GB** listings have traded toward the **upper** half of this band when storage is large and condition is excellent. |

### 2.2 Mac Studio — M3 Ultra, 512 GB RAM

| Channel | Approximate USD range | Notes |
|--------|------------------------|-------|
| **New (Apple BTO)** | **~$8,000 – $14,000+** | Press coverage of a **fully maxed** M3 Ultra Studio cited about **$14,099** including very large SSD upgrades ([MacRumors summary](https://www.macrumors.com/2025/03/05/maxed-out-m3-ultra-mac-studio-14099/)). A **512 GB RAM** configuration without max storage still lands in the **five figures** for typical BTO stacks. |
| **Used** | **~$7,500 – $13,000** | Low-volume, high-config machines show **volatile** resale; unusual listings (unopened, bundled AppleCare+, rare configs) swing the range. |

**Takeaway:** the **Studio class costs several times more** capital than a loaded **M3 Max notebook**, before electricity.

---

## 3. Depreciation and amortization assumptions

Hardware TCO for comparison needs a **holding period** and **residual value**.

### 3.1 Straight-line capital recovery

Let:

- \(P\) = purchase price  
- \(S\) = expected resale / salvage after \(T\) months  
- **Monthly capital cost** ≈ \((P - S) / T\)

Example (**36 months**, **45% residual** — plausible for in-demand Apple silicon if condition is good):

| Machine class | \(P\) (illustrative) | \(S \approx 0.45P\) | Net \(P-S\) | Monthly @ 36 mo |
|---------------|----------------------|---------------------|-------------|-----------------|
| MacBook M3 Max 128 GB | $4,800 | $2,160 | $2,640 | **$73** |
| Mac Studio M3 Ultra 512 GB | $11,000 | $4,950 | $6,050 | **$168** |

If resale is **worse** (30% residual), monthly capital rises roughly **+35–40%** versus the 45% case.

### 3.2 Opportunity cost (optional)

If capital earns risk-free yield \(r\) annually, the **economic** cost exceeds straight-line depreciation. This analysis **omits** financing and opportunity cost unless you explicitly model them—adding **~5–8%/year** on \(P\) materially increases the monthly “hurdle” API must clear.

---

## 4. Electricity cost

Local inference loads the SoC GPU for long stretches. Reasonable **order-of-magnitude** averages while the model is actually running (not idle desktop):

| Profile | Average system power (W) | Notes |
|---------|---------------------------|-------|
| MacBook Pro under sustained Metal LLM | **~55–90 W** | Fan curve and external display matter. |
| Mac Studio M3 Ultra under heavy GPU | **~120–220 W** | Thermal headroom higher; wall draw depends on screen peripherals. |

**Formula:**

\[
\text{Monthly kWh} = \frac{\text{Watts}}{1000} \times \text{hours/month}
\]
\[
\text{Monthly \$} = \text{kWh} \times \text{\$/kWh}
\]

Example: **70 W** average, **300 hours/month** GPU-busy, **$0.17/kWh** (rough US blended residential—your utility bill varies):

- kWh = \(0.07 \times 300 = 21\) → **$3.57/month**

Even aggressive **24/7** use at **120 W** is only ~**$62/month** at $0.17/kWh before demand charges (rare for homes). **Capital amortization dominates** TCO for these Macs—not wall power.

---

## 5. API spend model

Let:

- \(N_{\text{in}}\) = input tokens per month  
- \(f\) = fraction of input tokens billed as **cache miss** (the rest hit prefix cache)  
- \(N_{\text{out}}\) = output tokens per month  

Then (from the official price table):

\[
\text{API\$/month} =
\frac{N_{\text{in}}}{10^{6}}\bigl(0.14 f + 0.0028(1-f)\bigr)
+ \frac{N_{\text{out}}}{10^{6}}(0.28)
\]

**Important:** real agents often resend long prompts; DeepSeek’s **cache hit rate** can be high **when prefixes repeat**. That **shrinks** API bills and **weakens** the economic case for buying hardware purely to “save tokens.”

---

## 6. Break-even intuition (token volumes)

Define **hardware hurdle** \(H\) = monthly amortized capital **+** electricity (ignore labor, downtime, repairs).

Example hurdle for **MacBook** class: \(H \approx \$73 + \$4 \approx \$77\)/month (from §3.1 + moderate power).

Solve for token volumes where \(\text{API\$/month} = H\).

### 6.1 Output-dominated workload (expensive line item)

If input cost were negligible (unrealistic, but isolates output pricing):

\[
N_{\text{out}} > \frac{H}{0.28} \times 10^{6}
\]

For \(H = 77\): \(N_{\text{out}} > 275\text{M}\) **output tokens/month** (~**9.2M/day** sustained average).

At **Flash-class local decode** (~20–37 tok/s in README tables, workload-dependent), **one full-time stream** might produce on the order of **~2–6M tokens/day** if saturated—still **below** the pure output-token break-even against **only** ~$77/month. Once \(H\) rises (shorter amortization, worse resale, Studio capital), required volumes scale linearly.

### 6.2 Cache-friendly agent (most common economic pressure)

Suppose **90%** of input tokens are cache hits, **10%** miss, and **input:output = 10:1** by tokens (input-heavy coding agent):

Input $/M = \(0.1×0.14 + 0.9×0.0028 = 0.01652\)  
Per **1M output tokens**, associate **10M input tokens**:

\[
\text{\$/M output equivalent} = 10 × 0.01652 + 0.28 = 0.4452
\]

Break-even output tokens/month:

\[
N_{\text{out}} > \frac{H}{0.4452} × 10^{6}
\]

For \(H = 77\): \(N_{\text{out}} > 173\text{M}\)/month — still **very large** for an individual.

### 6.3 Worst case for API (always cache miss on input)

Same **10:1** input:output, **100%** cache miss on input:

\[
\text{\$/M output equivalent} = 10 × 0.14 + 0.28 = 1.68
\]
\[
N_{\text{out}} > \frac{77}{1.68} × 10^{6} \approx 45.8\text{M}/month
\]

That is **smaller** than the cache-friendly case but still **far beyond** typical personal coding usage.

**Conclusion of §6:** Against **listed Flash API** prices, **buying dedicated hardware to beat API $/token** usually requires **organization-scale** monthly traffic (millions to hundreds of millions of tokens, depending on cache behavior), **or** you must treat the Mac as **already purchased for other work** (see §7).

---

## 7. When local inference still wins (non-$/token or sunk hardware)

Pure token accounting understates why **ds4** exists. Common **non-economic** or **sunk-cost** advantages:

1. **Sunk hardware:** If the Mac is already owned for unrelated revenue work, the incremental monthly cost is roughly **electricity + wear**—the API break-even **volume drops** dramatically, but you still trade off **your time** (ops) and **peak throughput**.
2. **Privacy / compliance:** Data never leaves the device or controlled network.
3. **Offline / API outages / geopolitical access:** Local continues when bills, cards, or routes fail.
4. **Latency and UX:** Loopback HTTP + deterministic loading; no WAN jitter (still subject to local tok/s).
5. **Disk KV for huge prefixes:** The README emphasizes **stateless clients** re-sending massive prompts—ds4’s disk KV uses **local SSD** and avoids **re-paying cloud prefill** on every hop **without** relying on a vendor cache policy (you still paid for hardware).
6. **Model/version pinning:** Exact GGUF + engine pairing for reproducible agent behavior.

---

## 8. Throughput as a hidden cap

Even if API spend were high, **local generation speed** caps how many tokens you can produce per month per machine. README numbers are on the order of **~20–37 tok/s** generation (configuration-dependent). Cloud APIs can scale **horizontally** with separate billing; one **single-threaded Metal worker** in `ds4-server` processes **one** live session at a time (concurrent clients queue).

So “cheaper than API” and “faster than API” are **orthogonal**: economics might favor cloud at Flash pricing while **control** favors local.

---

## 9. Summary table (qualitative)

| Question | Typical answer at Flash list pricing |
|----------|--------------------------------------|
| Does wall power dominate Mac TCO? | **No** — **capital recovery** dominates. |
| Does prefix caching on API shrink savings from local? | **Yes** — high hit rates **reduce** cloud spend a lot. |
| Individual developer buying a Mac **only** to save API fees? | Usually **no** on pure $/token unless usage is **extreme**. |
| Org with **always-unique** giant contexts + huge outputs? | **Maybe** — run §5 with your measured \(f\), \(N_{\text{in}}\), \(N_{\text{out}}\). |
| Already-own hardware + privacy/offline needs? | **Local can dominate** on **requirements**, not nominal token breakeven. |

---

## 10. How to repeat this for your own numbers

1. Pick \(P\), \(S\), \(T\) for your hardware lifecycle.  
2. Estimate **GPU-busy hours/month** and **watts**; multiply by your **$/kWh**.  
3. Export token usage from your provider (split **input hit/miss** if available).  
4. Compute API $ with §5; compare to **(P−S)/T + electricity**.  
5. If close, add **ops labor**, **AppleCare**, **RAM/SSD upgrades**, and **financing**.

---

## References (stable URLs)

- DeepSeek API pricing: `https://api-docs.deepseek.com/quick_start/pricing`  
- Project README (hardware throughput table, KV format): [`README.md`](../README.md)  
- Mac Studio maxed pricing press note: `https://www.macrumors.com/2025/03/05/maxed-out-m3-ultra-mac-studio-14099/`  

# Learning Rust: My Journey from Literature to Distributed Systems

*Hi, I’m Emilie (Em'). This document summarizes how I moved from literature and languages to systems programming with Rust, then into formal data science studies, starting almost from scratch.*

---

## Where I Come From

Before 2025, my world was literature, linguistics, and foreign languages, not computers.  
When I first saw code, it looked like a tangle of acronyms and semicolons.

Curiosity pushed me to give programming a serious try.

---

## Early 2025: The Apple Foundation Program

**January/February 2025: I joined the Apple Foundation Program (AFP).**  
My first real encounter with code: Swift, UI/UX, Xcode, iOS apps.  
Everything felt visual and concrete: constructing, testing, deploying.  
Variables, loops, and functions started to make sense, like learning a new language to build things rather than only analyze them.

Most importantly, I realized I could learn to code.

---

## Spring–Summer 2025: Learning on My Own

After the AFP, I kept building Swift projects on my own.  
Bit by bit, the basics became clearer.  
One question kept coming back: *What really happens behind the scenes?*  
What do computers do with memory and files? How do real systems work?

---

## Autumn 2025: The Leap Into Rust

**Timeline:**
- **Started Rust:** October 27, 2025 (at 00:27 UTC+1 I ran my first "Hello World" in Rust)
- **Shipped mini-kvstore-v2:** November 21, 2025
- **Released minikv (distributed):** December 2025 (`v0.3.0` on December 22, then `v0.4.0` on December 31 with the first real admin dashboard and S3 API)
- **Started Data Science program at AMSE:** April 2, 2026 (Aix-Marseille School of Economics)

After hearing:
- “Rust is way too hard.”
- “Beware the borrow checker!”
- “It’s not for beginners.”

I had no formal tech background, but I wanted to understand how systems worked and challenge myself with low-level code.

---

## First Impressions

- **The compiler is strict but a true teacher:** error messages are detailed, sometimes even confessional—pointing to a solution.
- **Ownership and borrowing:** I thought I got “ownership” from literature, but Rust forces you to *internalize* it.
- **Everything’s explicit:** Who owns what, who can change or just borrow, and for how long.
- **The Rust community:** Genuinely welcoming, even to beginners.

---

## What Helped Me Along the Way

- **The Rust Book:** Everyone says it, because it’s true (especially Chapter 4—ownership!).
- **Clippy:** My favorite code reviewer, even when it stings.
- **Keeping notes:** Writing down every concept, compiler message, and solution helped me not get overwhelmed.
- **Building side projects:** Practice drives progress, including failed attempts.

---

## My Non-Tech Background: Actually an Advantage

- Loops, structure, types… remind me of literary analysis—except here it’s the machine that reads.
- Close reading (“is this reference mutable or immutable?”) and not skipping details—skills that transferred perfectly.
- Patience with ambiguity, digging deep until understanding—the same in both worlds.
- UI/UX taught me to design for people. Rust taught me to design for people *and* computers.

---

## What I Wish I Had Known Earlier

- *You don’t need to be “technical” to start.* Curiosity is the real prerequisite.
- *Don’t optimize too soon:* get it working, then get it right.
- *Testing can’t be too early.*
- *Learning isn’t linear.* There are setbacks and victories. Stick with it!

---

## Practical Tips

1. **Start before you feel “ready”**—you only get ready by doing.
2. **Read error messages like you’d read between the lines of a text**—all the clues are there.
3. **Celebrate every small win**—your first compiling program matters.
4. **Don’t be afraid to ask for help** (Discord, Reddit, Rust forums, etc.).
5. **Keep it enjoyable**: consistency is easier when you like the process.

---

## About minikv: What It Can Do (as of v1.0.0)

**Distributed Core:**
- Multi-node Raft consensus (leader election, log replication, snapshots, recovery, partition detection)
- Advanced Two-Phase Commit (2PC) for distributed writes: chunked transfers, error handling, retries, timeouts
- Configurable N-way replication (default: 3 replicas)
- High Random Weight (HRW) placement for even distribution
- 256 virtual shards for horizontal scaling
- Automatic cluster rebalancing (load detection, blob migration, metadata updates)
- Range queries (efficient key scans)
- Batch operations API (multi-put/get/delete)
- TLS encryption for HTTP and gRPC
- Flexible configuration: file, env, CLI override
- Admin dashboard endpoint (`/admin/status`) for cluster monitoring
- S3-compatible API (PUT/GET, in-memory and persistent backends)
- Watch/subscribe system (WebSocket and SSE) for real-time key change notifications

**Time Series and Vectors:**
- Time-series write and query APIs for event and metric workloads
- Query-time filtering and aggregation for analytical use cases
- Vector upsert and similarity query endpoints (top-k)
- Persistent vector index on coordinator disk for restart durability

**Storage Engine:**
- Segmented, append-only log structure
- In-memory HashMap indexing for O(1) key lookups

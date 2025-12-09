# Audio Architecture & Code Review

> **Review Date**: December 2025
> **Scope**: Audio Subsystem, Real-time Safety, Rust Idioms

## Executive Summary

The project demonstrates a high level of Rust proficiency. You are using modern tooling (`tokio`, `iced`, `sqlx`), excellent error handling patterns (`thiserror`/`anyhow`), and clean architecture (separation of concerns).

However, **there is a critical architectural flaw in the audio path** that violates real-time audio programming principles. While it may work on development machines, it is prone to "pops", "clicks", and audio glitches under load because of **priority inversion** and **memory allocation** in the real-time thread.

---

## 1. The "Rustacean" Style Check

### ✅ The Good

* **Error Handling:** Your separation of `thiserror` for library code and `anyhow` for the application/CLI layer is textbook idiomatic Rust.
* **Async/Sync Split:** You correctly identify that `cpal` and `symphonia` are synchronous/blocking by nature and wrap them in a dedicated thread, rather than trying to force them into `async` contexts.
* **Type Safety:** Strong typing in `PlayerState` and `PlaybackStatus` is good.
* **Modern Syntax:** You are using `let_chains` (`if let ... && let ...`), which implies you are on a very recent or nightly compiler (Rust 2024 edition). This makes for clean code.

### ⚠️ The "Un-Rust-Like" / Risky

* **`RwLock` in Real-Time Path:** This is the biggest issue (detailed below).
* **Implicit Panics:** In `src/ui/update.rs`, you use `.expect("Diagnostics task failed")`. If a thread panics, it's better to propagate that error to the UI state than to crash the task silently or panic the runtime.

---

## 2. Critical Audio Path Review

Your audio architecture is:
`Decoder Thread` -> `Crossbeam Channel` -> `CPAL Callback`

### 🔴 Critical Issue 1: Locks in the Audio Callback

In `src/player/audio.rs`, inside the `cpal` callback:

```rust
// ❌ DANGER: Taking a lock in a real-time audio callback
let volume = state.read().volume;
// ...
// ❌ DANGER: Taking a write lock in a real-time audio callback
state.write().position = chunk.timestamp;
```

**Why this is bad:**
The `cpal` callback runs on a high-priority system thread. `RwLock` (even `parking_lot`'s fast one) is a synchronization primitive. If the GUI thread (which runs at normal priority) holds this lock—for example, while rendering the UI or formatting a string—the audio thread will **block**.

* **Result:** The OS audio buffer runs dry -> Audible "pop" or glitch.
* **Fix:** Never lock in the callback. Use `std::sync::atomic` for simple values like `volume` (use `AtomicU32` with `f32::to_bits`) or `AtomicI64` for position.

### 🔴 Critical Issue 2: Allocations in the Audio Callback

You are sending `AudioChunk` over a channel. `AudioChunk` contains a `Vec<f32>`.

```rust
// src/player/audio.rs
struct AudioChunk {
    samples: Vec<f32>, // Heap allocation
    // ...
}
```

When the `cpal` callback finishes processing a chunk, that `AudioChunk` is dropped. Dropping a `Vec` triggers `free()` (deallocation).

**Why this is bad:**
Memory allocators (malloc/free) are non-deterministic and often use locks internally. Calling `free` in a real-time thread can take unpredictable amounts of time.

* **Fix:** Use a **Ring Buffer** (like `rtrb` or `ringbuf`) to stream `f32` samples directly. This involves zero allocation/deallocation in the hot path.

---

## 3. Proposed Audio Architecture (The "Rust Way")

To fix the issues above, I recommend refactoring `Player` to use **Lock-Free** structures.

### A. Shared State (Atomics)

Instead of `RwLock<PlayerState>`, split the state:

1. **UI State:** `Arc<RwLock<PlayerState>>` (Keep this for the UI, but don't read it in the audio thread).
2. **Audio Control:** A separate struct of atomics shared with the audio thread.

```rust
struct AudioSharedState {
    volume: AtomicU32, // Store f32 as bits
    is_playing: AtomicBool,
    // Position is tricky; usually better to send "Time Updates" back to UI via a channel
    // rather than writing to a shared atomic from the audio thread.
}
```

### B. Data Transport (Ring Buffer)

Replace `crossbeam_channel<AudioChunk>` with a lock-free ring buffer.

```rust
// In AudioOutput::new
let (mut producer, mut consumer) = rtrb::RingBuffer::<f32>::new(8192);
```

* **Decoder Thread:** Pushes `f32` samples into `producer`. If full, it waits/sleeps.
* **CPAL Callback:** Pops `f32` samples from `consumer`. If empty, it outputs silence (underrun).

### C. Message Passing for Events

* **To Audio:** `Sender<Command>` (Load, Seek, Stop).
* **From Audio:** `Sender<Event>` (PlaybackFinished, PositionUpdate(Duration), Underrun).

The UI thread reads `PositionUpdate` messages to update its `PlayerState` for display, rather than the audio thread writing directly to the state.

---

## 4. Edge Cases & Race Conditions

1. **The "Seek" Race:**
    * *Scenario:* User clicks "Seek to 50%".
    * *Current Code:* You send `Seek` command. The decoder seeks.
    * *Problem:* The `crossbeam` channel (or ring buffer) still contains 0.5 seconds of *old* audio (pre-seek).
    * *Result:* You hear 0.5s of old song, *then* it jumps.
    * *Fix:* When seeking, you must **flush** the ring buffer/channel. This is hard with channels (you have to drain them). With a ring buffer, you can just move the read pointer.

2. **The "Zombie" Stream:**
    * *Scenario:* You drop `AudioOutput`.
    * *Current Code:* `_audio_thread` is a `JoinHandle`.
    * *Problem:* If the decoder thread is blocked on `audio_tx.send()` (channel full) and you drop the receiver, the send will fail and the thread exits. This is actually handled correctly by your code (`send` returns error -> break loop). Good job!

3. **Visualization Latency:**
    * Your visualization runs in the decoder thread.
    * If the FFT takes too long (e.g., high-res FFT on a slow CPU), the decoder might fail to fill the audio buffer in time.
    * *Proposal:* Decouple visualization. Send raw audio samples to a *third* thread (or the UI thread) for FFT processing. Don't let eye-candy starve the ears.

---

## 5. Summary of Recommendations

1. **Immediate Fix:** Remove `RwLock` usage from `build_stream` closure. Use `AtomicU32` for volume.
2. **Performance Fix:** Switch from `Vec<f32>` channels to a `RingBuffer` for audio data.
3. **Correctness:** Implement a "Flush" mechanism for seeking to prevent playing stale audio buffers.
4. **Safety:** Ensure `diagnostics` and other background tasks handle panics gracefully without crashing the runtime.

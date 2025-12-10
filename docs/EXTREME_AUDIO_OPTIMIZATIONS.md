# Extreme Audio Path Optimizations

A deep dive into advanced techniques for achieving maximum performance in real-time audio processing. This document explains **why** each technique works, **how** to implement it, and **when NOT to use it**.

## Table of Contents

1. [SIMD Intrinsics](#1-simd-intrinsics)
2. [Inline Assembly](#2-inline-assembly)
3. [Memory-Mapped Ring Buffers](#3-memory-mapped-ring-buffers)
4. [GPU Compute for Visualization](#4-gpu-compute-for-visualization)
5. [Custom Arena Allocators](#5-custom-arena-allocators)
6. [Cache-Line Alignment](#6-cache-line-alignment)
7. [Branch-Free Audio Callbacks](#7-branch-free-audio-callbacks)
8. [NUMA-Aware Threading](#8-numa-aware-threading)
9. [Implementation Priority](#9-implementation-priority)

---

## 1. SIMD Intrinsics

### What is SIMD?

**Single Instruction, Multiple Data (SIMD)** allows a single CPU instruction to operate on multiple data elements simultaneously. Instead of processing one sample at a time, we process 4, 8, or 16 samples in parallel.

```text
Scalar (current):     [s0] → multiply → [s0*vol]
                      [s1] → multiply → [s1*vol]
                      [s2] → multiply → [s2*vol]
                      [s3] → multiply → [s3*vol]
                      = 4 instructions

AVX2 (proposed):      [s0,s1,s2,s3,s4,s5,s6,s7] → vmulps → [results]
                      = 1 instruction for 8 samples!
```

### x86-64 SIMD Instruction Sets

| Instruction Set | Register Width | Floats per Op | CPUs |
|----------------|----------------|---------------|------|
| SSE/SSE2 | 128-bit | 4 × f32 | All x86-64 (2003+) |
| AVX | 256-bit | 8 × f32 | Sandy Bridge+ (2011+) |
| AVX2 | 256-bit | 8 × f32 + FMA | Haswell+ (2013+) |
| AVX-512 | 512-bit | 16 × f32 | Skylake-X, Ice Lake+ |

### Why It Works for Audio

Audio processing is **embarrassingly parallel**:

- Each sample is independent
- Same operation (volume multiply) on all samples
- Data is contiguous in memory (cache-friendly)
- No data dependencies between samples

Our hot path does exactly this:

```rust
// Current scalar code
for sample in data.iter_mut() {
    *sample = T::from_sample(s * volume);  // One at a time
}
```

### How to Implement SIMD

Rust provides `std::arch` for portable SIMD intrinsics:

```rust
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

/// Apply volume to 8 samples at once using AVX2
#[target_feature(enable = "avx2")]
unsafe fn apply_volume_avx2(samples: &mut [f32], volume: f32) {
    let vol_vec = _mm256_set1_ps(volume);  // Broadcast volume to all 8 lanes
    
    let chunks = samples.len() / 8;
    for i in 0..chunks {
        let ptr = samples.as_mut_ptr().add(i * 8);
        let data = _mm256_loadu_ps(ptr);           // Load 8 floats
        let result = _mm256_mul_ps(data, vol_vec); // Multiply all 8
        _mm256_storeu_ps(ptr, result);             // Store 8 floats
    }
    
    // Handle remainder with scalar code
    let remainder_start = chunks * 8;
    for sample in &mut samples[remainder_start..] {
        *sample *= volume;
    }
}
```

### Performance Gains

Theoretical speedup: **8x for AVX2, 16x for AVX-512**

Real-world gains are typically 4-6x due to:

- Memory bandwidth limitations
- Remainder handling overhead
- Function call overhead for small buffers

### When NOT to Use SIMD

1. **Small buffers**: If processing <32 samples, scalar is often faster (setup overhead)
2. **Unaligned data**: Unaligned loads are slower (though modern CPUs handle this well)
3. **Complex conditionals**: If each sample needs different logic, SIMD doesn't help
4. **Portability requirements**: Not all CPUs support AVX2/AVX-512
5. **Maintenance burden**: SIMD code is harder to read and debug

### SIMD Verdict

Excellent fit for our volume multiplication and f32→i16 conversion hot paths.

### SIMD Use Cases

| Operation | SIMD Benefit | Priority |
|-----------|-------------|----------|
| Volume multiplication | ⭐⭐⭐⭐⭐ | High - hot path |
| f32→i16 conversion | ⭐⭐⭐⭐⭐ | High - hot path |
| FFT windowing | ⭐⭐⭐⭐ | Medium - per FFT frame |
| Sample interleaving | ⭐⭐⭐ | Medium - in decoder thread |
| Spectrum band averaging | ⭐⭐ | Low - small arrays |

---

## 2. Inline Assembly

### What is Inline Assembly?

Direct machine code embedded in Rust using the `asm!` macro. This gives complete control over:

- Register allocation
- Instruction scheduling
- Memory access patterns
- Branch prediction hints

### Why It Works

The compiler is good, but it can't always:

- Know your data access patterns
- Eliminate all redundant loads/stores
- Choose optimal instruction sequences
- Use specialized instructions (like prefetch)

### How to Implement

```rust
use std::arch::asm;

/// Ultra-fast volume application with manual register control
#[cfg(target_arch = "x86_64")]
unsafe fn apply_volume_asm(samples: &mut [f32], volume: f32) {
    let len = samples.len();
    let ptr = samples.as_mut_ptr();
    
    asm!(
        // Broadcast volume to ymm0
        "vbroadcastss ymm0, [{vol}]",
        
        // Main loop - process 8 samples per iteration
        "2:",
        "vmovups ymm1, [{ptr}]",           // Load 8 samples
        "vmulps ymm1, ymm1, ymm0",         // Multiply by volume
        "vmovups [{ptr}], ymm1",           // Store result
        "add {ptr}, 32",                   // Advance pointer by 8 floats
        "sub {len}, 8",                    // Decrement counter
        "jg 2b",                           // Loop if more samples
        
        ptr = inout(reg) ptr => _,
        len = inout(reg) len => _,
        vol = in(reg) &volume,
        options(nostack)
    );
}
```

### Advanced Techniques

**Prefetching**: Tell the CPU to load data before you need it:

```asm
prefetcht0 [ptr + 256]  ; Prefetch 256 bytes ahead into L1 cache
```

**Branchless operations**: Use conditional moves instead of jumps:

```asm
; Instead of: if (a > b) { result = a; } else { result = b; }
cmp eax, ebx
cmovg ecx, eax  ; Move if greater (no branch!)
cmovle ecx, ebx
```

**Instruction-level parallelism**: Interleave independent operations:

```asm
; CPU can execute these in parallel (different execution units)
vmulps ymm1, ymm1, ymm0  ; Uses multiply unit
vaddps ymm2, ymm2, ymm3  ; Uses add unit (can run simultaneously)
```

### When NOT to Use Assembly

1. **Compiler is usually better**: Modern compilers (LLVM) produce excellent code
2. **Portability nightmare**: x86-64 only, won't work on ARM
3. **Maintenance hell**: Very hard to read, debug, and modify
4. **Register allocation**: Manual allocation is error-prone
5. **Undefined behavior risk**: Easy to corrupt state
6. **Benchmarking required**: Must prove it's actually faster!

### Our Verdict

**Probably not worth it** for this project. The Rust compiler with `#[target_feature]` and intrinsics gets us 95% of the benefit with much better maintainability.

**Exception**: If profiling shows a specific 10-line hot spot where the compiler makes poor choices, a surgical assembly fix might help.

---

## 3. Memory-Mapped Ring Buffers

### The Problem with Traditional Ring Buffers

Standard ring buffers require **wrap-around handling**:

```rust
// Writing data that spans the end of the buffer
if write_pos + len > capacity {
    let first_part = capacity - write_pos;
    copy(&data[..first_part], &buffer[write_pos..]);  // First chunk
    copy(&data[first_part..], &buffer[..]);           // Wrap to start
} else {
    copy(&data, &buffer[write_pos..]);                // No wrap needed
}
```

This branching and double-copy hurts performance and complicates code.

### The Virtual Memory Trick

Map the **same physical memory twice**, contiguously in virtual address space:

```text
Physical memory:  [A][B][C][D]  (4 pages)

Virtual memory:   [A][B][C][D][A][B][C][D]
                  ^           ^
                  |           |
                  First mapping  Second mapping (same physical pages!)
```

Now you can write past the "end" and it automatically wraps:

```rust
// No wrap-around code needed!
// Writing at position 3 with length 3 just works:
copy(&data, &buffer[3..6]);  // Writes to D, then "A", "B" of second mapping
                              // Which IS the same as positions 3, 0, 1!
```

### How to Implement (Windows)

```rust
#[cfg(windows)]
mod mirrored_buffer {
    use windows_sys::Win32::System::Memory::*;
    use windows_sys::Win32::Foundation::*;
    
    pub struct MirroredBuffer {
        ptr: *mut u8,
        size: usize,
    }
    
    impl MirroredBuffer {
        pub fn new(size: usize) -> Result<Self, &'static str> {
            unsafe {
                // Size must be multiple of allocation granularity (64KB on Windows)
                let granularity = 65536;
                let size = (size + granularity - 1) & !(granularity - 1);
                
                // Create a file mapping (pagefile-backed)
                let mapping = CreateFileMappingW(
                    INVALID_HANDLE_VALUE,
                    std::ptr::null(),
                    PAGE_READWRITE,
                    0,
                    size as u32,
                    std::ptr::null(),
                );
                if mapping == 0 {
                    return Err("CreateFileMapping failed");
                }
                
                // Reserve virtual address space for TWO copies
                let placeholder = VirtualAlloc2(
                    0, // Current process
                    std::ptr::null(),
                    size * 2,
                    MEM_RESERVE | MEM_RESERVE_PLACEHOLDER,
                    PAGE_NOACCESS,
                    std::ptr::null(),
                    0,
                );
                if placeholder.is_null() {
                    return Err("VirtualAlloc2 failed");
                }
                
                // Split placeholder into two regions
                VirtualFree(placeholder, size, MEM_RELEASE | MEM_PRESERVE_PLACEHOLDER);
                
                // Map first copy
                let first = MapViewOfFile3(
                    mapping,
                    0,
                    placeholder,
                    0,
                    size,
                    MEM_REPLACE_PLACEHOLDER,
                    PAGE_READWRITE,
                    std::ptr::null(),
                    0,
                );
                
                // Map second copy (same physical pages!)
                let second = MapViewOfFile3(
                    mapping,
                    0,
                    placeholder.add(size),
                    0,
                    size,
                    MEM_REPLACE_PLACEHOLDER,
                    PAGE_READWRITE,
                    std::ptr::null(),
                    0,
                );
                
                CloseHandle(mapping);
                
                Ok(Self { ptr: first as *mut u8, size })
            }
        }
        
        /// Get a slice that can read past the "end" (wraps automatically)
        pub fn get_slice(&self, offset: usize, len: usize) -> &[u8] {
            let offset = offset % self.size;
            unsafe { std::slice::from_raw_parts(self.ptr.add(offset), len) }
        }
    }
}
```

### Performance Benefits

1. **Zero-copy reads/writes**: Never need to split operations
2. **Simpler code**: No wrap-around branches
3. **Better vectorization**: SIMD can process across "boundary"
4. **Cache-friendly**: Sequential access even at wrap point

### When NOT to Use

1. **Platform-specific**: Requires OS-specific APIs (Windows/Linux differ)
2. **Page alignment**: Buffer size must be multiple of page size (4KB/64KB)
3. **Limited virtual address space**: Uses 2x virtual memory
4. **Complexity**: Setup is complex, hard to debug
5. **Small buffers**: Overhead not worth it for <64KB buffers

### Our Use Case

Our ring buffer is 48,000 samples × 4 bytes = **192KB**. This is a good candidate!

However, the `rtrb` crate we use is already highly optimized. We should **benchmark first** to see if switching provides measurable benefit.

---

## 4. GPU Compute for Visualization

### Why GPU for Audio Visualization?

GPUs excel at:

- Massively parallel operations (thousands of cores)
- FFT algorithms (butterfly operations are parallel)
- Direct-to-screen rendering (skip CPU→GPU copy)

Our current visualization:

1. CPU computes FFT
2. CPU computes spectrum bands
3. CPU sends data to UI
4. GPU renders bars

With GPU compute:

1. CPU sends raw samples to GPU
2. GPU computes FFT
3. GPU computes spectrum bands
4. GPU renders directly (no copy!)

### How to Implement with wgpu

```rust
// Compute shader for FFT (simplified)
@group(0) @binding(0) var<storage, read_write> samples: array<f32>;
@group(0) @binding(1) var<storage, read_write> spectrum: array<f32>;

@compute @workgroup_size(256)
fn compute_fft(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    // FFT butterfly operations...
    // Each thread handles one frequency bin
}

@compute @workgroup_size(32)
fn compute_bands(@builtin(global_invocation_id) id: vec3<u32>) {
    let band = id.x;
    // Average bins for this band
    // Write to spectrum[band]
}
```

Rust integration:

```rust
use wgpu::*;

struct GpuVisualizer {
    device: Device,
    queue: Queue,
    sample_buffer: Buffer,
    spectrum_buffer: Buffer,
    fft_pipeline: ComputePipeline,
    band_pipeline: ComputePipeline,
}

impl GpuVisualizer {
    async fn compute_spectrum(&self, samples: &[f32]) -> Vec<f32> {
        // Write samples to GPU buffer
        self.queue.write_buffer(&self.sample_buffer, 0, bytemuck::cast_slice(samples));
        
        // Create command encoder
        let mut encoder = self.device.create_command_encoder(&Default::default());
        
        // FFT pass
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.fft_pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.dispatch_workgroups(samples.len() as u32 / 256, 1, 1);
        }
        
        // Band averaging pass
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.band_pipeline);
            pass.dispatch_workgroups(1, 1, 1);  // 32 bands
        }
        
        self.queue.submit([encoder.finish()]);
        
        // Read back spectrum (or better: render directly!)
        // ...
    }
}
```

### Performance Characteristics

| Approach | FFT Time (2048 samples) | Notes |
|----------|------------------------|-------|
| CPU (current) | ~50μs | Single-threaded |
| CPU SIMD | ~15μs | With AVX2 |
| GPU | ~5μs + 100μs overhead | Overhead dominates for small FFTs |

**Important**: GPU has high **latency** but high **throughput**. For a single 2048-sample FFT, CPU is faster! GPU wins when:

- Processing multiple FFTs in parallel
- Avoiding CPU→GPU data transfer (samples already on GPU)
- Rendering directly from compute output

### When NOT to Use GPU Compute

1. **Small workloads**: GPU kernel launch overhead (~100μs) dominates
2. **Data transfer costs**: Uploading samples may exceed compute savings
3. **Latency-sensitive**: GPU adds latency even if throughput is higher
4. **Complexity**: WGSL shaders, bind groups, pipeline setup
5. **Compatibility**: Some systems lack capable GPUs

### Our Verdict

**Not recommended** for our use case:

- Our FFT is 2048 samples, too small for GPU benefit
- We already have waveform on CPU (for other features)
- Adding GPU compute adds significant complexity

**Better approach**: Use SIMD-optimized CPU FFT (rustfft already does this!)

**Exception**: If we add real-time video visualization (like Winamp's Milkdrop), GPU compute makes sense for the shader effects.

---

## 5. Custom Arena Allocators

### The Problem with malloc/free

Standard allocation (`Vec::push`, `Box::new`) can:

1. **Block**: Global allocator lock contention
2. **Fragment**: Memory becomes scattered over time
3. **Be slow**: malloc is complex (free lists, coalescing)
4. **Page fault**: Growing allocations may trigger kernel

In real-time audio, any of these can cause **glitches**.

### What is an Arena Allocator?

Pre-allocate a chunk of memory, then "allocate" by just bumping a pointer:

```text
Traditional allocation:
┌─────────────────────────────────────────┐
│ malloc → search free list → split block → return │
└─────────────────────────────────────────┘
Time: ~100-1000ns, unpredictable

Arena allocation:
┌─────────────────────────┐
│ pointer += size; return │
└─────────────────────────┘
Time: ~1-5ns, constant
```

### How to Implement an Arena Allocator

```rust
/// Simple arena allocator for audio thread
pub struct AudioArena {
    buffer: Box<[u8]>,
    offset: usize,
}

impl AudioArena {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0u8; capacity].into_boxed_slice(),
            offset: 0,
        }
    }
    
    /// Allocate bytes (fast bump allocation)
    pub fn alloc(&mut self, size: usize, align: usize) -> Option<*mut u8> {
        // Align offset
        let aligned = (self.offset + align - 1) & !(align - 1);
        
        if aligned + size > self.buffer.len() {
            return None;  // Out of space
        }
        
        let ptr = unsafe { self.buffer.as_mut_ptr().add(aligned) };
        self.offset = aligned + size;
        Some(ptr)
    }
    
    /// Allocate a typed value
    pub fn alloc_val<T>(&mut self, val: T) -> Option<&mut T> {
        let ptr = self.alloc(std::mem::size_of::<T>(), std::mem::align_of::<T>())?;
        unsafe {
            let typed_ptr = ptr as *mut T;
            typed_ptr.write(val);
            Some(&mut *typed_ptr)
        }
    }
    
    /// Reset arena (instant "free all")
    pub fn reset(&mut self) {
        self.offset = 0;
    }
}

// Usage in audio thread
fn audio_thread_main() {
    let mut arena = AudioArena::new(1024 * 1024);  // 1MB pre-allocated
    
    loop {
        arena.reset();  // Free everything from last frame
        
        // All allocations this frame use the arena
        let samples = arena.alloc_slice::<f32>(4096);
        let spectrum = arena.alloc_slice::<f32>(32);
        
        // Process audio...
        
        // At end of frame, reset() frees everything instantly
    }
}
```

### Advanced: Per-Thread Arenas

```rust
thread_local! {
    static AUDIO_ARENA: RefCell<AudioArena> = RefCell::new(AudioArena::new(1024 * 1024));
}

/// Allocate from thread-local arena (no locking!)
pub fn audio_alloc<T>(val: T) -> &'static mut T {
    AUDIO_ARENA.with(|arena| {
        arena.borrow_mut().alloc_val(val).expect("arena full")
    })
}
```

### When NOT to Use Arenas

1. **Long-lived allocations**: Arena frees everything at once
2. **Variable lifetime**: Can't free individual allocations
3. **Unknown size**: Must pre-allocate maximum needed
4. **Shared data**: Thread-local arenas can't share allocations
5. **Complexity**: Managing arena lifetime adds code

### Our Use Case

Our current code is already **nearly allocation-free** in the hot path:

- Ring buffer is pre-allocated
- FFT buffers are pre-allocated
- We use `Vec::with_capacity` where needed

**Recommendation**: Profile first. If we find allocations in the hot path, introduce arenas for those specific cases.

---

## 6. Cache-Line Alignment

### How CPU Caches Work

Modern CPUs have hierarchical caches:

```
CPU Core
├── L1 Cache (32KB, ~4 cycles latency)
├── L2 Cache (256KB, ~12 cycles latency)  
├── L3 Cache (8MB shared, ~40 cycles latency)
└── RAM (~200+ cycles latency)
```

Data is loaded in **cache lines** (64 bytes on x86-64). Even if you read 1 byte, the CPU loads 64 bytes.

### False Sharing

When two CPU cores modify variables in the **same cache line**, they fight:

```text
Core 1: writes to variable A ─┐
                              ├─ Same cache line!
Core 2: writes to variable B ─┘

Result: Cache line bounces between cores ("ping-pong")
        Each write invalidates the other core's cache
        Performance tanks!
```

### How to Implement

```rust
/// Pad struct to fill entire cache line
#[repr(C, align(64))]  // 64-byte alignment
pub struct CacheLineAligned<T> {
    value: T,
    _padding: [u8; 64 - std::mem::size_of::<T>()],  // Fill rest of cache line
}

// Better: Use crossbeam's CachePadded
use crossbeam_utils::CachePadded;

pub struct AudioSharedState {
    // Each atomic on its own cache line - no false sharing!
    volume: CachePadded<AtomicU32>,
    position: CachePadded<AtomicU64>,
    is_playing: CachePadded<AtomicBool>,
    buffer_fill: CachePadded<AtomicU32>,
}
```

### Aligning Arrays for SIMD

AVX2 loads are fastest when aligned to 32 bytes:

```rust
#[repr(C, align(32))]
pub struct AlignedBuffer {
    data: [f32; 1024],
}

// Or use aligned_vec crate
use aligned_vec::{AVec, ConstAlign};
type Align32 = ConstAlign<32>;

let buffer: AVec<f32, Align32> = AVec::new(32);
```

### When NOT to Use Alignment

1. **Wastes memory**: Padding increases memory usage
2. **May hurt cache**: Fewer items fit in cache if padded
3. **Single-threaded code**: False sharing only matters multi-threaded
4. **Read-mostly data**: False sharing is a write problem
5. **Already cache-line sized**: No need to over-align

### Our Use Case

Our `AudioSharedState` has atomics accessed from multiple threads. We should:

1. **Audit current state struct**: Check if atomics share cache lines
2. **Add padding if needed**: Use `CachePadded` from crossbeam
3. **Align sample buffers**: For SIMD operations

---

## 7. Branch-Free Audio Callbacks

### Why Branches Hurt

Modern CPUs use **speculative execution**: they guess which way a branch will go and start executing ahead. If wrong:

- Pipeline flush (~15-20 cycles penalty)
- Wasted work thrown away
- Unpredictable latency

For audio with tight deadlines, unpredictable latency = glitches.

### Identifying Problematic Branches

Our current callback:

```rust
if !is_playing {           // Branch 1: Usually not taken
    // fill with silence
    return;
}

for sample in data.iter_mut() {
    match consumer.pop() {  // Branch 2: Usually Ok, occasionally Err
        Ok(s) => *sample = ...,
        Err(_) => {
            audio_shared.increment_underruns();  // Rare
            *sample = 0.0;
        }
    }
}
```

Branch 1 is predictable (usually playing). Branch 2 could mispredict on underruns.

### Branch-Free Techniques

**1. Conditional Select (cmov)**

```rust
// Instead of:
if condition { a } else { b }

// Use:
let mask = if condition { !0u32 } else { 0u32 };
let result = (a & mask) | (b & !mask);

// Or with intrinsics:
use std::arch::x86_64::*;
let result = _mm256_blendv_ps(b_vec, a_vec, mask_vec);
```

**2. Multiply by 0 or 1**

```rust
// Instead of:
if is_playing { sample * volume } else { 0.0 }

// Use:
let playing_f32 = is_playing as u32 as f32;  // 0.0 or 1.0
sample * volume * playing_f32
```

**3. Lookup Tables**

```rust
// Instead of:
match state {
    Playing => handle_playing(),
    Paused => handle_paused(),
    Stopped => handle_stopped(),
}

// Use function pointer table:
const HANDLERS: [fn(&mut Self); 3] = [
    Self::handle_playing,
    Self::handle_paused,
    Self::handle_stopped,
];
HANDLERS[state as usize](self);
```

**4. SIMD Masking**

```rust
// Process all samples, then mask out paused ones
let samples = _mm256_loadu_ps(ptr);
let scaled = _mm256_mul_ps(samples, volume_vec);
let playing_mask = _mm256_set1_ps(is_playing as f32);  // All 0s or all 1s
let result = _mm256_mul_ps(scaled, playing_mask);
_mm256_storeu_ps(ptr, result);
```

### When NOT to Go Branch-Free

1. **Predictable branches**: If branch is 99%+ predictable, CPU handles it fine
2. **Complex logic**: Branch-free code is hard to read and maintain
3. **Error handling**: Can't make error paths branch-free (nor should you)
4. **Readability**: `if` is clearer than bit manipulation
5. **No measured benefit**: Profile first!

### Our Use Case

The main branch in our callback (`is_playing`) is highly predictable. The underrun branch is rare and acceptable.

**Recommendation**: Only go branch-free for SIMD loops where we need to handle edge cases (like volume ramping to prevent clicks).

---

## 8. NUMA-Aware Threading

### What is NUMA?

**Non-Uniform Memory Access (NUMA)**: In multi-socket systems, each CPU has "local" memory that's faster than "remote" memory on other CPUs.

```text
┌─────────────────┐     ┌─────────────────┐
│     CPU 0       │     │     CPU 1       │
│  Cores 0-7      │     │  Cores 8-15     │
│  ┌───────────┐  │     │  ┌───────────┐  │
│  │  Memory   │◄─┼──┬──┼─►│  Memory   │  │
│  │  (Fast)   │  │  │  │  │  (Fast)   │  │
│  └───────────┘  │  │  │  └───────────┘  │
└─────────────────┘  │  └─────────────────┘
                     │
              Interconnect
              (Slower cross-access)
```

### Thread Affinity

Pin a thread to specific CPU core(s):

```rust
#[cfg(windows)]
fn set_thread_affinity(core: usize) {
    use windows_sys::Win32::System::Threading::*;
    unsafe {
        let mask = 1u64 << core;
        SetThreadAffinityMask(GetCurrentThread(), mask);
    }
}

#[cfg(target_os = "linux")]
fn set_thread_affinity(core: usize) {
    use libc::{cpu_set_t, sched_setaffinity, CPU_SET, CPU_ZERO};
    unsafe {
        let mut set: cpu_set_t = std::mem::zeroed();
        CPU_ZERO(&mut set);
        CPU_SET(core, &mut set);
        sched_setaffinity(0, std::mem::size_of::<cpu_set_t>(), &set);
    }
}
```

### Real-Time Thread Priority

Elevate audio thread priority:

```rust
#[cfg(windows)]
fn set_realtime_priority() {
    use windows_sys::Win32::System::Threading::*;
    unsafe {
        // WASAPI's Audio Engine uses MMCSS - we can too
        let mut task_index: u32 = 0;
        AvSetMmThreadCharacteristicsW(
            "Pro Audio\0".encode_utf16().collect::<Vec<_>>().as_ptr(),
            &mut task_index,
        );
    }
}

#[cfg(target_os = "linux")]
fn set_realtime_priority() {
    use libc::{sched_param, sched_setscheduler, SCHED_FIFO};
    unsafe {
        let param = sched_param { sched_priority: 50 };
        sched_setscheduler(0, SCHED_FIFO, &param);
    }
}
```

### Memory Allocation Awareness

Allocate memory on the correct NUMA node:

```rust
#[cfg(windows)]
fn alloc_on_node(size: usize, node: u32) -> *mut u8 {
    use windows_sys::Win32::System::Memory::*;
    unsafe {
        VirtualAllocExNuma(
            GetCurrentProcess(),
            std::ptr::null_mut(),
            size,
            MEM_RESERVE | MEM_COMMIT,
            PAGE_READWRITE,
            node,
        ) as *mut u8
    }
}
```

### When NOT to Use NUMA Awareness

1. **Single-socket systems**: Most desktops are single-socket (no NUMA)
2. **Small data**: NUMA effects negligible for small working sets
3. **Complexity**: Thread affinity can hurt if set wrong
4. **Scheduler knows best**: OS scheduler is usually smarter than us
5. **Portability**: APIs differ per OS

### Our Use Case

Most music-minder users have desktop/laptop systems (single NUMA node). However:

1. **Thread affinity**: Pinning audio thread to one core prevents migration (good!)
2. **Realtime priority**: **Definitely useful** - prevents other tasks from interrupting audio
3. **NUMA memory**: Not applicable for most users

**Recommendation**: Implement realtime priority (cpal may already do this). Thread affinity is optional.

---

## 9. Implementation Priority

Based on effort vs. impact:

### High Priority (Do First)

| Technique | Effort | Impact | Risk |
|-----------|--------|--------|------|
| SIMD volume/conversion | Medium | High | Low |
| Cache-line alignment | Low | Medium | Low |
| Realtime thread priority | Low | Medium | Low |

### Medium Priority (Do If Needed)

| Technique | Effort | Impact | Risk |
|-----------|--------|--------|------|
| Memory-mapped ring buffer | High | Medium | Medium |
| Arena allocators | Medium | Low | Low |
| Branch-free hot paths | Medium | Low | Medium |

### Low Priority (Probably Skip)

| Technique | Effort | Impact | Risk |
|-----------|--------|--------|------|
| Inline assembly | Very High | Low | High |
| GPU compute | Very High | Low* | High |
| NUMA-aware allocation | High | Very Low | Medium |

*GPU compute becomes high impact only for complex visualizations

---

## Implementation Roadmap

### Phase 1: SIMD Foundation

1. Create `src/player/simd.rs` module
2. Implement AVX2 volume multiplication
3. Implement AVX2 f32→i16 conversion  
4. Add runtime CPU feature detection
5. Benchmark against scalar baseline

### Phase 2: Memory Optimization  

1. Audit `AudioSharedState` for false sharing
2. Add `CachePadded` wrappers where needed
3. Align sample buffers for SIMD
4. Benchmark cache effects

### Phase 3: Thread Priority

1. Research cpal's existing priority handling
2. Add MMCSS integration on Windows
3. Add SCHED_FIFO on Linux (if root/CAP_SYS_NICE)
4. Test for glitch reduction under load

### Phase 4: Advanced (Optional)

1. Prototype mirrored ring buffer
2. Benchmark against rtrb
3. Implement only if measurable improvement

---

## Benchmarking Strategy

Before implementing any optimization:

```rust
#[cfg(test)]
mod benchmarks {
    use std::time::Instant;
    
    fn benchmark<F: FnMut()>(name: &str, iterations: u32, mut f: F) {
        // Warmup
        for _ in 0..100 { f(); }
        
        // Measure
        let start = Instant::now();
        for _ in 0..iterations { f(); }
        let elapsed = start.elapsed();
        
        println!("{}: {:?} per iteration", 
            name, 
            elapsed / iterations
        );
    }
    
    #[test]
    fn bench_volume_scalar_vs_simd() {
        let mut samples = vec![0.5f32; 4096];
        let volume = 0.8f32;
        
        benchmark("scalar", 10000, || {
            for s in &mut samples { *s *= volume; }
        });
        
        benchmark("avx2", 10000, || {
            unsafe { apply_volume_avx2(&mut samples, volume); }
        });
    }
}
```

Use `cargo bench` with criterion for proper statistical analysis:

```toml
[dev-dependencies]
criterion = "0.5"

[[bench]]
name = "audio_benchmarks"
harness = false
```

---

## References

- [Intel Intrinsics Guide](https://www.intel.com/content/www/us/en/docs/intrinsics-guide/index.html)
- [Rust std::arch documentation](https://doc.rust-lang.org/std/arch/index.html)
- [What Every Programmer Should Know About Memory](https://people.freebsd.org/~lstewart/articles/cpumemory.pdf)
- [LLVM Auto-vectorization](https://llvm.org/docs/Vectorizers.html)
- [Lock-Free Programming](https://www.cs.cmu.edu/~410-s05/lectures/L31_LockFree.pdf)
- [Real-Time Audio Programming 101](http://www.rossbencina.com/code/real-time-audio-programming-101-time-waits-for-nothing)

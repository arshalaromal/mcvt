# MCVT v1.0 Developer Documentation

**Architecture, Data Flow, and Technical Debt**

---

## 1. System Overview

MCVT (Multi-format ConVerTer) is a Rust-based CLI application that functions as a routing and execution layer over three external C/C++ binaries: FFmpeg, ImageMagick, and Pandoc.

The primary engineering objective of this system is to abstract the differing command-line syntax, error handling, and file parsing logic of these three tools into a single, unified interface. It is designed to handle highly concurrent directory-level processing while maintaining strict memory safety and preventing host OS resource starvation.

---

## 2. Architecture Rationale

The architecture decisions in MCVT were driven by the need to handle unsafe external processes (C-binaries) safely within a multithreaded Rust environment.

### 2.1 File Identification via Magic Bytes (`router.rs`)

Relying on file extensions is inherently fragile for media processing. `router.rs` utilizes the `file_format` crate to read the binary header of the input file. This allows the system to accurately map the input to a logical `Domain` (VideoAudio, Image, or Document) even if the file extension is spoofed or missing. Extension parsing via `mime_guess` is only used as a fallback or for determining the target output domain.

### 2.2 Thread-Safe Dependency Caching (`executor.rs`)

In batch processing mode, the `rayon` worker pool spins up $N$ concurrent threads (where $N$ is the number of logical CPU cores). Each thread must verify that the underlying binary (e.g., `ffmpeg`) exists.

* **The Problem:** Spawning thousands of `std::process::Command::new("ffmpeg").arg("--version")` calls concurrently will cause a denial-of-service (DOS) condition on the OS process scheduler.
* **The Solution:** A globally static `OnceLock<RwLock<HashSet<String>>>` is implemented. The `RwLock` allows infinite concurrent threads to read the cache memory simultaneously. The actual system check is performed once per binary, written to the heap, and subsequent thread requests return `Ok(())` in constant time ($O(1)$) with zero I/O overhead.

### 2.3 Pre-Calculated Thread Delegation (`batch.rs`)

When processing a directory containing thousands of files, passing ownership of the state to closures can cause severe memory bloat.
`batch.rs` loops over the directory *single-threadedly* first. It calculates all relative path structures, creates required output directories dynamically, and skips self-overwrites. It then passes a lightweight tuple of `(PathBuf, PathBuf)` to the `rayon::par_iter()` pool. This ensures threads spend zero CPU cycles on path string manipulation and are strictly dedicated to executing binaries.

### 2.4 Orphaned Process Assassination (`main.rs`)

External binaries like FFmpeg intercept `SIGINT` (Ctrl+C) signals to handle graceful shutdowns. If a user interrupts MCVT, the Rust process terminates immediately, leaving detached FFmpeg processes running in the background (zombies).
To prevent hardware resource locking, `main.rs` initializes a global `ctrlc` handler combined with the `sysinfo` crate. Upon receiving an interrupt, MCVT scans the entire OS process tree, identifies any process where `parent() == Some(MCVT_PID)`, executes a hard kill command on those children, and then exits safely.

---

## 3. Execution Data Flow

1. **CLI Parsing:** `clap` parses arguments into the `Cli` struct in `main.rs`.
2. **Execution Branching:**
* **Single-File:** Validates canonical paths to prevent identical `input == output` self-overwrites (which truncates source files). Triggers `indicatif` spinner.
* **Batch-Mode:** `batch.rs` utilizes `walkdir` for recursive mapping, pre-calculates target extensions, and instantiates the `rayon` pool.


3. **Domain Routing:** `router.rs` evaluates input/output and returns a tuple: `(Domain, Domain)`.
4. **Task Orchestration:** `executor.rs` matches the domain tuple to a specific execution pipeline (e.g., `(Domain::Image, Domain::VideoAudio)` maps to the Image-to-Video pipeline).
5. **Template Injection:** Default arguments (e.g., `bwdif` de-interlacing, H.264 codecs) are pulled from `templates.rs` unless overridden by namespaced CLI args (`--ffmpeg-out`).
6. **Binary Execution:** The specific Runner (`FfmpegRunner`, `ImageMagickRunner`, `PandocRunner`) executes the command and monitors `stderr`.

---

## 4. Current Technical Debt and Sub-Optimal Code Areas

While the system is functionally stable, several areas of technical debt exist in the current v1.0 codebase.

### 4.1 Runner Struct Redundancy

**Location:** `src/runners/*.rs`
**Issue:** `FfmpegRunner`, `ImageMagickRunner`, and `PandocRunner` contain nearly identical implementations. They all hold `binary_path`, `input_args`, `output_args`, `verbose`, and a `run()` method.
**Refactoring Required:** These structs violate DRY principles. A `ToolRunner` Trait should be defined, and a generic execution struct should handle the `Command::new` instantiation, with specific tools simply implementing a method to structure their unique argument order (e.g., Pandoc requiring `-o`).

### 4.2 Cache Stampede Race Condition

**Location:** `src/executor.rs` (`check_dependency`)
**Issue:** The `RwLock` prevents the 10,000-file DOS issue, but a minor race condition exists on the initial cycle. When the batch processor starts, $N$ threads hit the `cache.read()` lock at the exact same time. Seeing the cache is empty, all $N$ threads drop the read lock and proceed to execute the `--version` system check simultaneously before the first thread can acquire the write lock.
**Refactoring Required:** Implement an atomic state machine (e.g., `Unchecked`, `Pending`, `Verified`) inside the cache. If a thread sees `Pending`, it yields until the state updates, ensuring the system check is only ever executed exactly once.

### 4.3 Forced Overwrites in Batch Mode

**Location:** `src/batch.rs` and `src/runners/ffmpeg.rs`
**Issue:** `batch.rs` checks if an output file already exists and prints `[!] Warning: Overwriting existing file`. However, the execution logic does not pause or prompt the user. Because `ffmpeg.rs` hardcodes the `-y` flag, the existing file is immediately destroyed.
**Refactoring Required:** Implement a `--skip-existing` flag in the CLI, or handle a prompt request. Currently, the warning is informational but unpreventable.

### 4.4 Naive Template Management

**Location:** `src/templates.rs`
**Issue:** The optimization templates (e.g., `scale=trunc(iw/2)*2:trunc(ih/2)*2`) are hardcoded directly into the Rust binaries. If a user wishes to permanently alter the default CRF value from 16 to 18, they must recompile the entire source code.
**Refactoring Required:** Extract template generation into a deserialized configuration file (e.g., `~/.config/mcvt/templates.toml`).

### 4.5 Panic via Expect on UI Components

**Location:** `src/batch.rs` and `src/main.rs`
**Issue:** The progress bar instantiation relies on `.expect("Invalid progress bar template")`. While the string is hardcoded and currently safe, any future malformation in the template syntax during updates to the `indicatif` crate will result in a hard panic, crashing the application rather than propagating a clean error state up the stack.
# MCVT v1.0 User Manual

**Multi-format ConVerTer**

---

## 1. Overview and Purpose

MCVT (Multi-format ConVerTer) is a unified command-line interface engineered to standardize media and document conversions. System administrators and power users traditionally rely on disparate utilities—such as FFmpeg for media, ImageMagick for images, and Pandoc for documents—each possessing a unique syntax, dependency chain, and error-handling methodology.

MCVT successfully abstracts these underlying complexities. It dynamically routes inputs to the appropriate backend processor utilizing MIME-type detection and file-header ("magic byte") analysis, applies context-aware optimization templates, and executes operations with strict memory safety. The architecture features a thread-safe parallel processing engine designed for massive directory batches, dynamic folder restructuring, and intelligent dependency caching.

### 1.1 Core Capabilities

* **Unified Routing:** Automatically identifies input and output domains (Video/Audio, Image, Document) and invokes the correct underlying binary without user intervention.
* **Magic Byte Detection:** Bypasses false or spoofed extensions (e.g., a `.png` maliciously renamed to `.docx`) by reading the file's binary header to ensure precise routing.
* **Thread-Safe Batch Processing:** Utilizes a concurrent worker pool to process entire directory trees simultaneously, scaling workload distribution exactly to the host machine's available CPU cores.
* **Recursive Directory Mirroring:** Scans deep nested folder hierarchies and perfectly replicates the structural layout in the designated output destination.
* **Process Lifecycle Management:** Intercepts system interrupts (e.g., `Ctrl+C`) to trace and terminate orphaned background processes (zombies), ensuring CPU threads are released cleanly.
* **Targeted Argument Injection:** Permits advanced users to bypass default optimization templates by injecting raw, granular arguments directly into specific backend processors.

---

## 2. System Prerequisites

MCVT operates as an advanced routing and execution layer. It requires the underlying backend binaries to be installed and globally accessible within the system `PATH`, or explicitly defined via command-line arguments.

* **FFmpeg:** Required for Video and Audio domains.
* **ImageMagick (`magick`):** Required for Image domains and PDF rasterization.
* **Pandoc:** Required for Document domains. *(Note: PDF generation via Pandoc requires a local LaTeX engine, such as `pdflatex` or `xelatex`)*.

---

## 3. Standard Operations (Single File)

Single-file operations require exactly two positional arguments: the input file path and the target output file path.

**Syntax:**

```bash
mcvt <INPUT_FILE> <OUTPUT_FILE>

```

### 3.1 Execution Examples

* **Video Format Conversion:**
```bash
mcvt source.mp4 final.mkv

```


* **Audio Format Conversion:**
```bash
mcvt recording.wav track.mp3

```


* **Image Format Conversion:**
```bash
mcvt graphic.png compressed.jpg

```


* **Document Format Conversion:**
```bash
mcvt essay.docx essay.pdf

```



> **Note:** MCVT will dynamically create the output directory tree if the specified destination path does not currently exist.

---

## 4. Batch Operations (Directory Processing)

When a directory path is passed as the input argument, MCVT automatically engages Batch Mode. The batch controller calculates all required relative paths, checks for potential file collisions, and executes conversions in parallel utilizing all available CPU threads.

**Required Flag:**

* `--batch-ext <EXTENSION>`: Specifies the target format for all processed files in the batch (e.g., `mkv`, `jpg`, `pdf`).

**Optional Flags:**

* `--batch-in <EXTENSION>`: Filters the input directory to strictly process files matching this extension.
* `-R` or `--recursive`: Instructs the engine to scan all subdirectories and replicate the complete folder tree in the target output directory.

### 4.1 Execution Examples

* **Standard Batch Conversion:** Convert all compatible media within a single folder to MKV.
```bash
mcvt ./raw_footage/ ./encoded_footage/ --batch-ext mkv

```


* **Filtered Batch Conversion:** Convert *only* PNG files to JPG.
```bash
mcvt ./assets/ ./processed/ --batch-ext jpg --batch-in png

```


* **Recursive Batch Conversion:** Convert all files across a deeply nested project directory, maintaining the exact folder layout.
```bash
mcvt ./project_root/ ./project_export/ --batch-ext mp4 -R

```



---

## 5. Overrides and Routing Control

The MCVT router relies on file inspection to determine the correct execution binary. When standard routing fails or non-standard behavior is explicitly required, users may override the engine's automated logic.

### 5.1 The `--force` Override

Forces the router to execute a specific inter-domain pathway, bypassing all automated detection protocols. The format requires passing the exact `input_domain:output_domain`. Valid domains are `video`, `audio`, `image`, and `document`.

* **Example:** Forcing ImageMagick to attempt processing a video file.
```bash
mcvt animation.mp4 frames.pdf --force video:document

```



### 5.2 The `--no-guess` Flag

Disables "Magic Byte" binary header inspection. The engine will rely strictly on the provided file extension string. This is vital for processing raw data streams or intentionally malformed files.

* **Example:**
```bash
mcvt corrupted.jpg restored.png --no-guess

```



---

## 6. Advanced Argument Injection (Namespacing)

MCVT automatically applies heavily optimized default templates to standard conversions. However, power users frequently require granular control over parameters such as bitrate, scaling, or filtering. Namespaced flags allow raw arguments to be passed directly to the underlying binaries, completely overriding MCVT's templates.

**Available Injection Flags:**

* `--ffmpeg-in` / `--ffmpeg-out`
* `--magick-in` / `--magick-out`
* `--pandoc-in` / `--pandoc-out`

### 6.1 Execution Examples

* **Direct FFmpeg Video Encoding Control:** Bypassing default templates to force a specific bitrate and scale constraints.
```bash
mcvt input.mp4 output.mkv --ffmpeg-out -b:v 1M -vf scale=1280:720

```


* **Direct FFmpeg Stream Copy:** Transferring container formats without re-encoding, resulting in zero CPU overhead and zero quality loss.
```bash
mcvt input.mkv output.mp4 --ffmpeg-out -c copy

```


* **Direct Pandoc Engine Selection:**
```bash
mcvt doc.docx doc.pdf --pandoc-out --pdf-engine=xelatex

```


* **Restricting CPU Threads in Batch:** Preventing FFmpeg from overloading the OS CPU scheduler during massive parallel batch operations.
```bash
mcvt ./in/ ./out/ --batch-ext mkv --ffmpeg-out -threads 1

```



---

## 7. Custom Executable Paths

If the backend binaries are not located in the global system `PATH`, or if the user is testing experimental versions of the binaries, explicit file paths can be mapped to the execution runners.

**Available Path Flags:**

* `--ffmpeg-path <PATH>`
* `--magick-path <PATH>`
* `--pandoc-path <PATH>`
* **Example:**
```bash
mcvt in.mp4 out.mp4 --ffmpeg-path /opt/custom_builds/ffmpeg_v7

```



---

## 8. Built-in Optimization Templates

Unless explicitly overridden by Namespaced arguments (Section 6), MCVT automatically applies the following logic chains to ensure baseline production quality and prevent pipeline failures.

### 8.1 Video Constraints (FFmpeg)

When outputting to `.mp4` or `.mkv`:

* Forces H.264 encoding (`libx264`).
* Sets CRF (Constant Rate Factor) to 16 for near-lossless quality retention.
* Injects a De-interlacing filter (`bwdif`) to eliminate motion blur from interlaced source material.
* Forces frame dimensions to be mathematically divisible by 2 via `scale=trunc(iw/2)*2:trunc(ih/2)*2` (prevents catastrophic hardware decoding crashes on odd-pixel dimensions).
* Applies the `+faststart` flag to shift metadata for immediate web playback compatibility.

### 8.2 Inter-Domain Handling

* **Video to GIF:** Bypasses standard encoding and injects a 2-pass `palettegen` and `paletteuse` complex filter to render high-quality, dithered animations locked at 15 FPS.
* **Static Image to Video:** Applies a `-loop 1` instruction, forcing a static image to output as a continuous 5-second video track (prevents media player crashes caused by 1-frame/0-second videos).
* **PDF to Image:** Injects `-density 300` into the input read stage, ensuring crisp, high-resolution document rasterization.
* **Image to PDF:** Injects `-resize 1200x1200>` to prevent multi-gigabyte document generation when wrapping raw, high-megapixel photography.

---

## 9. System Safety and Error Handling

### 9.1 Verbose Debugging (`-v` or `--verbose`)

By default, backend process logs are truncated to their final three lines to protect terminal rendering and progress bar integrity. The `--verbose` flag overrides this muzzle, forcing the engine to dump the complete, raw standard error (`stderr`) stream to the console for deep debugging.

```bash
mcvt input.mp4 output.mkv -v

```

### 9.2 Orphaned Process Termination

Batch processing initiates multiple simultaneous, CPU-intensive C-binaries. If MCVT is terminated abruptly via a keyboard interrupt (`Ctrl+C`), the engine pauses its own shutdown sequence, scans the entire OS process tree, and executes a kill signal to all child processes it generated. This guarantees zero detached background threads (zombies) are left to silently consume system resources.

### 9.3 Self-Overwrite Protection

MCVT mathematically verifies canonical paths prior to execution. If the input file and the target output file resolve to the identical disk sector, the engine immediately aborts the operation. This critical safety check prevents backend binaries (such as FFmpeg) from truncating the source file to zero bytes during the file-opening phase. In Batch Mode, conflicting files are automatically skipped and flagged, allowing the broader batch execution to continue uninterrupted.
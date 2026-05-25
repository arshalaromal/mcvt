# MCVT (Multi-Format File Conversion Engine)

A high-performance, offline, open-source multi-format file conversion engine built in Rust. MCVT rejects the monolithic approach, utilizing a decoupled, domain-driven architecture where file processing is delegated to independent modules coordinated by a central Orchestrator.

## Architectural Layout

```text
mcvt/
├── Cargo.toml
└── src/
    ├── main.rs               # Driver / CLI Harness
    ├── lib.rs                # Public API Surface
    ├── core/                 # Engine Core Logic
    │   ├── mod.rs
    │   ├── orchestrator.rs   # Type probing, bypass fallback, and routing
    │   ├── command.rs        # Fluent API for secure process execution
    │   └── env.rs            # Sandbox & scratchpad manager
    └── modules/              # Domain-Specific Transformers
        ├── mod.rs
        ├── video.rs          # FFmpeg Engine Interface
        ├── image.rs          # ImageMagick Engine Interface
        └── document.rs       # Pandoc HTML-IR Engine Interface
```

## Technical Core Principles

* **Zero-Monolith Coupling:** The Orchestrator does not know *how* a file is processed. It maps a domain to a handler and hands off execution.
* **"Let It Crash" Diagnostics:** No massive, custom internal parsers. External tool failures capture `stderr` natively and bubble the raw context straight to the user.
* **Format Agnostic Routing:** File types are verified via magic bytes using `file-format`. An explicit override bypass allows processing when magic bytes are missing or corrupted.
* **Shell-Injection Proof:** The internal `CommandBuilder` completely avoids raw shell evaluation (`sh -c`), executing primitives via vector arguments directly through the OS kernel.

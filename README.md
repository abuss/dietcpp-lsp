# DietC++ LSP

A Language Server Protocol implementation for DietC++ - a restricted, modern C++ subset that enforces best practices and forbids unsafe or legacy C++ features.

## Overview

DietC++ is a coding standard enforced in real-time via LSP. It promotes modern C++ practices by detecting and flagging violations of the subset rules as you type.

### What DietC++ forbids

- Virtual functions (prefer templates / composition)
- `goto` statements
- Preprocessor directives (`#include`, `#define`, etc.)
- Raw pointers without smart pointer wrappers
- Traditional C-style for loops (prefer range-based for)

## Features

- **Real-time violation detection** - errors appear inline as you type
- **Quick fixes** - auto-fix violations via LSP code actions
- **Configurable rules** - customize which violations are reported via `.dietcpprc.json` or `.dietcpprc.toml`
- **Multi-format config** - supports both JSON and TOML configuration files
- **Symbol table & AST** - full parser with symbol resolution

## Project Structure

```
dietcpp-lsp/
├── src/
│   ├── main.rs          # LSP server entry point
│   ├── lib.rs           # Module declarations
│   ├── ast.rs           # Abstract syntax tree definitions
│   ├── parser.rs        # C++ parser
│   ├── token.rs         # Tokenizer / lexer
│   ├── violations.rs    # Constraint violation detection
│   ├── symbol_table.rs  # Symbol table management
│   ├── lsp.rs           # LSP server implementation
│   ├── code_actions.rs  # LSP code actions (quick fixes)
│   └── config.rs        # Configuration system
└── Cargo.toml
```

## Installation

### Build from source

```bash
cargo build --release
```

The binary will be available at `target/release/dietcpp-lsp`.

### CLI support (optional)

```bash
cargo build --release --features cli
```

## Usage

Run the LSP server (stdio transport):

```bash
./target/release/dietcpp-lsp
```

The server reads and writes JSON-RPC messages over stdin/stdout, conforming to the Language Server Protocol specification.

## Configuration

Create a configuration file in your project root:

### JSON format (`.dietcpprc.json`)

```json
{
  "general": { "enabled": true, "severity": "warning" },
  "rules": {
    "preprocessor_directives": true,
    "forbidden_keywords": true,
    "traditional_for_loops": true,
    "raw_pointers": true
  },
  "features": { ... },
  "naming": { ... }
}
```

### TOML format (`.dietcpprc.toml`)

```toml
[general]
enabled = true
severity = "warning"

[rules]
preprocessor_directives = true
forbidden_keywords = true
traditional_for_loops = true
raw_pointers = true
```

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `serde` | 1.0 | Serialization / deserialization |
| `serde_json` | 1.0 | JSON config parsing |
| `toml` | 0.8 | TOML config parsing |
| `lsp-types` | 0.95 | LSP protocol types |
| `clap` | 4.4 | CLI parsing (optional feature) |
| `log` / `env_logger` | 0.4 / 0.11 | Logging |

## Testing

```bash
cargo test --lib --release
```

## License

MIT License - see [LICENSE](./LICENSE) file for details.

## Author

Antal A. Buss

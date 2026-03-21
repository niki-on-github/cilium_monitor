# Cilium Monitor - Agent Guidelines

## Build & Development

### Prerequisites
- Use `nix-portable nix develop . --no-write-lock-file` to enter the development shell
- The shell provides: Rust, Cargo, protobuf compiler, and required dependencies

### Verification (Always use nix-portable)
**IMPORTANT**: Always verify code using `nix-portable` to ensure proper build environment with protoc:

```bash
# Verify code compiles (required before any changes)
nix-portable nix develop . --no-write-lock-file --command cargo check

# Verify code compiles with bash wrapper (alternative)
nix-portable nix develop . --no-write-lock-file --command bash -c 'cargo check'

# Clean and verify (use when build issues occur)
nix-portable nix develop . --no-write-lock-file --command bash -c 'cargo clean && cargo check'

# Full verification: check, format, and lint
nix-portable nix develop . --no-write-lock-file --command bash -c 'cargo check && cargo fmt --check && cargo clippy'
```

**Note**: Do NOT run cargo commands directly without nix-portable - protoc will not be available and proto compilation will fail.

### Build Commands
```bash
# Check code compiles (fast)
nix-portable nix develop . --no-write-lock-file --command cargo check

# Build release binary
nix-portable nix develop . --no-write-lock-file --command cargo build --release

# Build debug binary
nix-portable nix develop . --no-write-lock-file --command cargo build

# Run the binary
nix-portable nix develop . --no-write-lock-file --command ./target/debug/cilium_monitor

# Run with custom address and port
nix-portable nix develop . --no-write-lock-file --command ./target/debug/cilium_monitor --address 10.0.1.11 --port 31234

# Run with verbosity level (minimal, normal, verbose)
nix-portable nix develop . --no-write-lock-file --command ./target/debug/cilium_monitor --verbosity normal

# Disable colors
nix-portable nix develop . --no-write-lock-file --command ./target/debug/cilium_monitor --no-color
```

### Linting & Formatting
```bash
# Format code
cargo fmt --check          # Check formatting
cargo fmt                  # Auto-format

# Lint code
cargo clippy               # Run clippy linter
cargo clippy --fix         # Auto-fix clippy warnings
```

### Testing
```bash
# Run all tests
cargo test

# Run a specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Run tests in a specific module
cargo test --lib module_name
```

## Code Style Guidelines

### Imports
- Group imports: external crates first, then internal modules
- Use `use` statements at the top of each file before module definitions
- Prefer specific imports over glob imports (`use x::Y` not `use x::*`)
- Group related imports together with blank lines between groups

Example:
```rust
use clap::Parser;
use tonic::Request;

use api::observer::GetFlowsRequest;
```

### Formatting
- Use `cargo fmt` for consistent formatting
- 4-space indentation (Rust default)
- Blank lines between logical sections (imports, modules, functions)
- Keep lines under 100 characters when reasonable
- Use trailing commas in multi-line function calls

### Type Conventions
- Use `String` for dynamic strings, `&str` for references
- Prefer `Result<T, E>` over `Option<T>` when errors are possible
- Use `Box<dyn Error>` for generic error types in async functions
- Use explicit types for complex return values
- Prefer `#[derive(Debug)]` for structs used in logging

### Naming Conventions
- **Structs/Enums**: PascalCase (`GetFlowsRequest`)
- **Functions/variables**: snake_case (`parse_args`, `endpoint`)
- **Modules**: snake_case (`observer_pb`, `flow_pb`)
- **Constants**: SCREAMING_SNAKE_CASE (`MAX_BUFFER_SIZE`)
- **CLI args**: kebab-case in flags (`--address`, `--port`)

### Error Handling
- Use `?` operator for propagating errors in async functions
- Return `Result<(), Box<dyn std::error::Error>>` from main
- Use `unwrap_or_else()` with meaningful defaults
- Add comments explaining error conditions when unclear

Example:
```rust
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ObserverClient::connect(endpoint).await?;
    Ok(())
}
```

### Async Code
- Always use `#[tokio::main]` for async main functions
- Use `.await` immediately after async calls
- Prefer `tokio::sync` primitives for concurrency
- Use `Result` for fallible async operations

### Comments
- Use `//` for inline comments
- Add comments for non-obvious logic (e.g., proto module structure)
- Use numbered comments for sequential steps in main functions
- Document CLI argument purposes with inline comments

### Proto Integration
- Proto files compiled via `tonic-build` in `build.rs`
- Generated code uses `tonic::include_proto!()` macro
- Proto modules must be nested to handle cross-references
- Use `prost-types` for well-known types (Timestamp, Any, etc.)

### CLI Arguments
- Use `clap` with derive macros
- Define `#[derive(Parser)]` structs for CLI args
- Use `#[arg(long, default_value = "...")]` for options
- Provide meaningful help text via `#[command(author, version, about)]`

## Project Structure
```
/workspace
├── Cargo.toml          # Dependencies and metadata
├── build.rs            # Proto compilation
├── flake.nix           # Nix development environment
├── proto/              # .proto source files
│   ├── observer/
│   ├── flow/
│   └── relay/
└── src/
    └── main.rs         # Application entry point
```

## Key Dependencies
- **tokio**: Async runtime with `rt-multi-thread`, `macros` features
- **tonic**: gRPC client framework (v0.11)
- **prost**: Protobuf encoding/decoding
- **clap**: CLI argument parsing with derive feature
- **prost-types**: Protobuf well-known types

## Common Tasks

### Update proto files
1. Modify `.proto` files in `proto/` directory
2. Rebuild: `cargo build` (triggers `build.rs`)
3. Update imports in `main.rs` if struct paths change

### Add new dependency
1. Add to `[dependencies]` in `Cargo.toml`
2. Run `cargo check` to verify

### Debug build issues
1. Check proto paths in `build.rs`
2. Verify `protoc` is available in nix shell
3. Check for missing `prost-types` dependency for well-known types

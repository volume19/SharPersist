# Build Instructions

## Prerequisites

### Required

- Rust 1.70 or later
- For Windows builds: Visual Studio 2019+ with C++ tools or Build Tools for Visual Studio

### Development Tools (Optional)

```bash
# Install clippy and rustfmt
rustup component add clippy rustfmt

# Install cargo-audit for security scanning
cargo install cargo-audit

# Install cargo-deny for license checking
cargo install cargo-deny
```

## Building

### Debug Build

```bash
cargo build --workspace
```

Binary location: `target/debug/sharpersist` (or `.exe` on Windows)

### Release Build

```bash
cargo build --release --workspace
```

Binary location: `target/release/sharpersist` (or `.exe` on Windows)

Release builds are optimized with:
- Full optimizations (`opt-level = 3`)
- Link-time optimization (LTO)
- Single codegen unit for maximum optimization
- Debug symbols stripped
- Panic unwinding disabled (`panic = abort`)

### Windows MSVC Target (Cross-compilation)

```bash
# Add target
rustup target add x86_64-pc-windows-msvc

# Build
cargo build --release --target x86_64-pc-windows-msvc
```

### Linux Target

```bash
# Most techniques are Windows-only, but the binary compiles with stubs
cargo build --release --target x86_64-unknown-linux-gnu
```

## Running Tests

### All Tests

```bash
cargo test --workspace
```

### Unit Tests Only

```bash
cargo test --lib
```

### Integration Tests (Windows Only, May Require Admin)

```bash
# Run with sequential execution to avoid conflicts
cargo test --test '*' -- --test-threads=1
```

### Specific Module Tests

```bash
cargo test windows_registry
cargo test techniques::registry
```

## Code Quality Checks

### Format Check

```bash
cargo fmt --all -- --check
```

### Linting

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

### Security Audit

```bash
cargo audit
```

### License Check

```bash
cargo deny check licenses
```

## Reproducible Builds

For reproducible builds, pin dependency versions in `Cargo.lock` and use identical Rust toolchain versions.

## Platform-Specific Notes

### Windows
- Requires Windows SDK for registry and service APIs
- Some operations require administrator privileges
- Test in a VM or isolated environment

### Linux
- Builds successfully with Windows-specific code stubbed out
- Techniques return `PlatformNotSupported` errors
- Useful for development and testing error paths

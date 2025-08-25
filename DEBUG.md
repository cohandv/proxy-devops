# Debugging Guide for Proxy CLI

## 1. Logging-based Debugging

### Setting Log Levels
Use the `RUST_LOG` environment variable to control log output:

```bash
# Show only errors and warnings
RUST_LOG=warn cargo run -- --target http://example.com

# Show info, warn, and error messages
RUST_LOG=info cargo run -- --target http://example.com

# Show all debug messages (most verbose)
RUST_LOG=debug cargo run -- --target http://example.com

# Show logs for specific modules only
RUST_LOG=proxy=debug cargo run -- --target http://example.com
```

### Log Levels in Code
- `debug!()` - Detailed tracing information
- `info!()` - General information about program execution
- `warn!()` - Potentially problematic situations
- `error!()` - Error conditions

## 2. Cargo Debug Commands

```bash
# Build with debug symbols (default for dev profile)
cargo build

# Verbose build output
cargo build --verbose

# Check for common issues
cargo clippy

# Run tests
cargo test

# Check for formatting issues
cargo fmt --check

# Format code
cargo fmt
```

## 3. VS Code Debugging

Install the "rust-analyzer" and "CodeLLDB" extensions, then:

1. Open the project in VS Code
2. Set breakpoints by clicking in the gutter
3. Press F5 or go to Run > Start Debugging
4. Choose "Debug proxy" configuration

### Debug Configurations Available:
- **Debug proxy**: Runs with sample arguments (`--target http://example.com --port 3000 --verbose`)
- **Debug proxy (no args)**: Runs without arguments (will show help or error)

## 4. Command Line Debugging

### Using LLDB (macOS/Linux)
```bash
# Start debugger
lldb target/debug/proxy

# In LLDB prompt:
(lldb) breakpoint set --name main
(lldb) run --target http://example.com --port 3000
(lldb) continue
(lldb) frame variable
(lldb) quit
```

### Using GDB (Linux)
```bash
gdb target/debug/proxy
(gdb) break main
(gdb) run --target http://example.com --port 3000
(gdb) continue
(gdb) print port
(gdb) quit
```

## 5. Print Debugging Techniques

### Simple Print Statements
```rust
println!("Debug: variable = {:?}", variable);
println!("Debug: reached checkpoint A");
```

### Debug Formatting
```rust
#[derive(Debug)]
struct MyStruct {
    field: String,
}

let my_var = MyStruct { field: "test".to_string() };
println!("Debug: {:?}", my_var);  // Debug format
println!("Pretty: {:#?}", my_var); // Pretty debug format
```

### Conditional Debugging
```rust
if verbose {
    println!("Debug info only shown when verbose flag is set");
}
```

## 6. Panic and Error Debugging

### Better Panic Messages
```rust
// Add to Cargo.toml for better panic backtraces
[profile.dev]
panic = "abort"

# Run with backtrace
RUST_BACKTRACE=1 cargo run -- --target http://example.com
RUST_BACKTRACE=full cargo run -- --target http://example.com
```

### Using anyhow for Better Error Messages
```rust
use anyhow::{Context, Result};

fn parse_port(port_str: &str) -> Result<u16> {
    port_str.parse::<u16>()
        .context("Failed to parse port number")
}
```

## 7. Performance Debugging

```bash
# Profile the application
cargo install flamegraph
cargo flamegraph --bin proxy -- --target http://example.com

# Memory usage analysis
cargo install valgrind
valgrind target/debug/proxy --target http://example.com
```

## 8. Common Debugging Scenarios

### Command Line Argument Issues
- Use `dbg!()` macro to print values: `dbg!(&matches);`
- Check if arguments are being parsed correctly
- Verify required arguments are provided

### Network/IO Issues
- Use `RUST_LOG=debug` to see detailed network activity
- Check if ports are available: `lsof -i :8080`
- Verify target URLs are reachable: `curl -I http://example.com`

### Build Issues
- Clean build artifacts: `cargo clean && cargo build`
- Update dependencies: `cargo update`
- Check for conflicting versions: `cargo tree`

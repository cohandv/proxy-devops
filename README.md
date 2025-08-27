# Proxy - A Pluggable Command-Line Tool

A extensible Rust-based command-line proxy tool with a dynamic plugin system. Easily add new functionality through loadable plugins.

## 🚀 Quick Start

### Building the Project

```bash
# Clone and build the main proxy tool and all plugins
cargo build --release

# Or build in debug mode for development
cargo build
```

### Running

```bash
# Run the main proxy tool
./target/release/proxy --help

# Example: Use the k8s_port_forward plugin
./target/release/proxy k8s_port_forward --help
```

## 📁 Project Structure

```
proxy/
├── src/                    # Main proxy CLI application
│   └── main.rs
├── plugin_api/            # Plugin API definition
│   ├── Cargo.toml
│   └── src/lib.rs
├── plugins/               # Individual plugins
│   └── k8s_port_forward/  # Kubernetes port forwarding plugin
│       ├── Cargo.toml
│       └── src/lib.rs
└── Cargo.toml            # Workspace configuration
```

## 🔌 Plugin System

### How Plugins Work

1. **Dynamic Loading**: Plugins are compiled as dynamic libraries (`.dylib` files)
2. **Plugin Discovery**: The main CLI automatically discovers and loads plugins from the plugin directory
3. **Subcommand Integration**: Each plugin registers itself as a subcommand
4. **Configuration**: Plugins can have their own configuration files

### Plugin Directory

Plugins are loaded from:
- **Environment Variable**: `$PROXY_PLUGIN_DIR`
- **Default**: `~/.cohandv/proxy/plugins/`

### Configuration Directory

Plugin configurations are stored in:
- **Environment Variable**: `$PROXY_PLUGINS_CONFIG_DIR`
- **Default**: `~/.cohandv/proxy/config/plugins.d/`

## 🛠️ Creating a New Plugin

### 1. Plugin Structure

Create a new plugin directory under `plugins/`:

```bash
mkdir plugins/my_plugin
cd plugins/my_plugin
```

### 2. Cargo.toml

Create a `Cargo.toml` with the plugin API dependency:

```toml
[package]
name = "my_plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]  # Required for dynamic loading

[dependencies]
plugin_api = { path = "../../plugin_api" }
clap = { version = "4", features = ["derive"] }
# Add other dependencies as needed
```

### 3. Plugin Implementation

Create `src/lib.rs`:

```rust
use plugin_api::Plugin;
use clap::{Command, Arg, ArgMatches};

pub struct MyPlugin;

impl Plugin for MyPlugin {
    fn name(&self) -> &'static str {
        "my_plugin"
    }

    fn subcommand(&self) -> Command {
        Command::new(self.name())
            .about("Description of what my plugin does")
            .arg(
                Arg::new("option")
                    .long("option")
                    .help("An example option")
                    .required(false)
            )
    }

    fn run(&self, matches: &ArgMatches) {
        println!("My plugin is running!");

        if let Some(option_value) = matches.get_one::<String>("option") {
            println!("Option value: {}", option_value);
        }
    }
}

// Required: Export function for dynamic loading
#[no_mangle]
pub extern "C" fn create_plugin() -> Box<dyn Plugin> {
    Box::new(MyPlugin)
}
```

### 4. Add to Workspace

Update the main `Cargo.toml` to include your plugin:

```toml
[workspace]
members = [
    "plugin_api",
    "plugins/k8s_port_forward",
    "plugins/my_plugin",  # Add this line
]
```

### 5. Build and Test

```bash
# Build your plugin
cargo build

# The plugin will be automatically available
./target/debug/proxy my_plugin --help
```

## 📋 Available Plugins

### k8s_port_forward

Kubernetes port forwarding plugin that supports both name-based and label-based targeting.

#### Configuration

Create `~/.cohandv/proxy/config/plugins.d/k8s_port_forward.conf`:

```toml
# Port forward by name
[[forward]]
name = "my-service"
namespace = "default"
type = "service"
local_port = 8080
remote_port = 80

# Port forward by labels (will use first matching resource)
[[forward]]
labels = "app=nginx,version=v1"
namespace = "default"
type = "pod"
local_port = 9090
remote_port = 9000

# Another example by name
[[forward]]
name = "my-pod"
namespace = "default"
type = "pod"
local_port = 3000
remote_port = 3000
```

#### Usage

```bash
# Forward all configured ports
./target/release/proxy k8s_port_forward

# Forward specific configuration by name
./target/release/proxy k8s_port_forward --name my-service

# Forward specific configuration by label matching
./target/release/proxy k8s_port_forward --name nginx
```

#### Features

- **Name-based targeting**: Direct resource name specification
- **Label-based targeting**: Automatically finds first matching resource
- **Multiple resource detection**: Shows all matches when using labels
- **Blocking execution**: Keeps port forwarding active until Ctrl+C
- **Graceful termination**: Properly handles cleanup on exit

## 🔧 Plugin Configuration

### Configuration Files

Each plugin can have its own configuration file following the pattern:
- **File**: `~/.cohandv/proxy/config/plugins.d/{plugin_name}.conf`
- **Format**: TOML (recommended)

### Reading Configuration

Use the plugin API helper function:

```rust
use plugin_api::plugin_config_path;
use std::fs;

fn load_config(plugin_name: &str) -> Option<MyConfig> {
    let config_path = plugin_config_path(plugin_name)?;
    let content = fs::read_to_string(config_path).ok()?;
    toml::from_str(&content).ok()
}
```

## 🏗️ Development

### Prerequisites

- Rust 1.70+ (for latest features)
- Cargo

### Development Workflow

1. **Make changes** to plugin code
2. **Build**: `cargo build`
3. **Test**: `./target/debug/proxy plugin_name --help`
4. **Debug**: Use standard Rust debugging tools

### VSCode Configuration

The project includes launch configurations for debugging:

```json
{
    "type": "codelldb",
    "request": "launch",
    "name": "Debug proxy",
    "program": "${workspaceFolder}/target/debug/proxy",
    "args": ["--help"],
    "cwd": "${workspaceFolder}"
}
```

## 📦 Deployment

### Installing Plugins

1. **Build**: `cargo build --release`
2. **Copy plugins**: Copy `.dylib` files from `target/release/` to plugin directory
3. **Configure**: Set up configuration files as needed

### Plugin Distribution

Plugins can be distributed as:
- **Source code**: Users build locally
- **Compiled libraries**: Distribute `.dylib` files directly
- **Package managers**: Future integration with cargo/homebrew

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Add your plugin or improvements
4. Test thoroughly
5. Submit a pull request

### Plugin Guidelines

- **Follow naming conventions**: Use snake_case for plugin names
- **Add comprehensive help**: Use clap's help system effectively
- **Handle errors gracefully**: Provide meaningful error messages
- **Document configuration**: Include example config files
- **Test thoroughly**: Ensure plugin works in various scenarios

## 📝 API Reference

### Plugin Trait

```rust
pub trait Plugin {
    fn name(&self) -> &'static str;
    fn subcommand(&self) -> Command;
    fn run(&self, matches: &ArgMatches);
}
```

### Utility Functions

```rust
// Get plugin configuration path
pub fn plugin_config_path(plugin_name: &str) -> Option<PathBuf>
```

## 🐛 Troubleshooting

### Plugin Not Loading

- Check plugin is in correct directory
- Verify `.dylib` extension
- Ensure `create_plugin` function is exported
- Check for dependency conflicts

### Configuration Issues

- Verify config file path and permissions
- Check TOML syntax
- Ensure required fields are present

### Runtime Errors

- Check plugin dependencies are installed
- Verify external tool availability (e.g., `kubectl`)
- Review log output for detailed errors

## 📄 License

[Add your license information here]

## 🙋 Support

- **Issues**: [GitHub Issues](link-to-issues)
- **Discussions**: [GitHub Discussions](link-to-discussions)
- **Documentation**: This README and inline code documentation

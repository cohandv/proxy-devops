use clap::{Arg, Command};
use libloading::{Library, Symbol};
use plugin_api::Plugin;
use std::fs;
use std::path::PathBuf;

/// Proxy CLI
fn main() {
    // Determine plugin directory from environment variable or default
    let plugin_dir = std::env::var_os("PROXY_PLUGIN_DIR")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".cohandv/proxy/plugins")))
        .expect("Could not determine plugin directory");

    println!("Loading plugins from: {}", plugin_dir.display());

    let mut app = Command::new("proxy")
        .version("0.1.0")
        .about("A command line proxy tool")
        .arg(
            Arg::new("list-plugins")
                .long("list-plugins")
                .help("List all available plugins with their versions")
                .action(clap::ArgAction::SetTrue),
        );

    let mut plugins = Vec::new();

    if let Ok(entries) = fs::read_dir(&plugin_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("dylib") {
                // Optionally skip known non-plugin dylibs
                if let Some(fname) = path.file_name().and_then(|s| s.to_str()) {
                    if fname == "libplugin_api.dylib" {
                        continue;
                    }
                }
                unsafe {
                    let lib = Library::new(&path).unwrap();
                    let constructor: Result<Symbol<unsafe extern "C" fn() -> Box<dyn Plugin>>, _> =
                        lib.get(b"create_plugin");
                    if let Ok(constructor) = constructor {
                        let plugin = constructor();
                        app = app.subcommand((*plugin).subcommand());
                        plugins.push((lib, plugin)); // Keep lib alive!
                    }
                }
            }
        }
    }

    let mut app_clone = app.clone();
    let matches = app.get_matches();

    // Handle --list-plugins flag
    if matches.get_flag("list-plugins") {
        println!();
        println!("📦 Available Plugins:");
        println!();

        if plugins.is_empty() {
            println!("❌ No plugins found in: {}", plugin_dir.display());
            println!();
            println!("💡 To install plugins:");
            println!("   1. Download plugin .dylib/.so/.dll files");
            println!("   2. Copy to: {}", plugin_dir.display());
            println!("   3. Run: proxy --list-plugins");
        } else {
            println!("┌──────────────────────┬────────────┬──────────────────────────────────┐");
            println!("│ Plugin Name          │ Version    │ Description                      │");
            println!("├──────────────────────┼────────────┼──────────────────────────────────┤");

            for (_, plugin) in &plugins {
                let name = plugin.name();
                let version = plugin.version();
                let description = plugin.description();

                // Truncate description if too long
                let desc_truncated = if description.len() > 32 {
                    format!("{}...", &description[..29])
                } else {
                    description.to_string()
                };

                println!(
                    "│ {:<20} │ {:<10} │ {:<32} │",
                    name, version, desc_truncated
                );
            }

            println!("└──────────────────────┴────────────┴──────────────────────────────────┘");
            println!();
            println!("💡 Usage: proxy <plugin-name> --help");
            println!("📋 Example: proxy k8s_port_forward --help");
        }

        println!();
        println!("📂 Plugin directory: {}", plugin_dir.display());
        return;
    }

    // Handle plugin subcommands
    for (_, plugin) in plugins {
        if let Some(sub_m) = matches.subcommand_matches(plugin.name()) {
            (*plugin).run(sub_m);
            return;
        }
    }

    // If no plugin matched and no special flags, show help
    if matches.subcommand_name().is_none() {
        let _ = app_clone.print_help();
        println!("\n\n💡 Use --list-plugins to see available plugins");
    }
}

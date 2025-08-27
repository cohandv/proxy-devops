use clap::Command;
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
        .about("A command line proxy tool");

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

    let matches = app.get_matches();

    for (_, plugin) in plugins {
        if let Some(sub_m) = matches.subcommand_matches(plugin.name()) {
            (*plugin).run(sub_m);
        }
    }
}

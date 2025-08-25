use clap::{Command, Parser};
use libloading::{Library, Symbol};
use plugin_api::Plugin;
use std::fs;
use std::path::PathBuf;

/// Proxy CLI
#[derive(Parser)]
struct Args {
    /// Plugin directory
    #[arg(long, env = "PROXY_PLUGIN_DIR")]
    plugin_dir: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();
    let plugin_dir = args.plugin_dir
        .or_else(|| {
            dirs::home_dir().map(|h| h.join(".cohandv/proxy/plugins"))
        })
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
                println!("Loading plugin: {}", path.display());
                let lib = Library::new(&path).unwrap();
                unsafe {
                    let constructor: Symbol<unsafe extern fn() -> Box<dyn Plugin>> =
                        lib.get(b"create_plugin").unwrap();
                    let plugin = constructor();
                    app = app.subcommand(plugin.subcommand());
                    plugins.push((lib, plugin)); // Keep lib alive!
                }
            }
        }
    } else {
        println!("No plugins found in {}", plugin_dir.display());
    }

    let matches = app.get_matches();

    for (_, plugin) in plugins {
        if let Some(sub_m) = matches.subcommand_matches(plugin.name()) {
            plugin.run(sub_m);
        }
    }
}

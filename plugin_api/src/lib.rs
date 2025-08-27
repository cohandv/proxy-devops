use std::path::PathBuf;
/// Returns the config path for a given plugin name, e.g. ~/.cohandv/proxy/config/plugins.d/{plugin_name}.conf
pub fn plugin_config_path(plugin_name: &str) -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("PROXY_PLUGINS_CONFIG_DIR") {
        Some(PathBuf::from(dir).join(format!("{plugin_name}.conf")))
    } else {
        dirs::home_dir().map(|h| {
            h.join(".cohandv/proxy/config/plugins.d")
                .join(format!("{plugin_name}.conf"))
        })
    }
}
use clap::{ArgMatches, Command};

pub trait Plugin {
    fn name(&self) -> &'static str;
    fn version(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn subcommand(&self) -> Command;
    fn run(&self, matches: &ArgMatches);
}

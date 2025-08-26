// --- Module scope ---
use plugin_api::Plugin;
use clap::{Command, Arg, ArgMatches};
use log::{debug, info, warn, error};
use std::fs;
use std::process::Command as ProcessCommand;
use std::process::Stdio;
use serde::Deserialize;
use kube::{Client, api::Api};
use tokio::runtime::Runtime;
use k8s_openapi::api::core::v1::Pod;

#[derive(Debug, Deserialize)]
pub struct ForwardConfig {
    pub forward: Vec<PortForward>,
}

#[derive(Debug, Deserialize)]
pub struct PortForward {
    pub name: String,
    pub namespace: String,
    pub r#type: String, // "pod" or "service"
    pub local_port: u16,
    pub remote_port: u16,
}

pub struct ProxyPlugin;

impl ProxyPlugin {
    /// Returns a sample config file for this plugin (TOML format)
    pub fn sample_config() -> &'static str {
        r#"[[forward]]
name = "my-service"
namespace = "default"
type = "service"
local_port = 8080
remote_port = 80

[[forward]]
name = "my-pod"
namespace = "default"
type = "pod"
local_port = 9090
remote_port = 9000
"#
    }
}

fn load_config(plugin_name: &str) -> Option<ForwardConfig> {
    let config_path = plugin_api::plugin_config_path(plugin_name)?;
    let content = fs::read_to_string(config_path).ok()?;
    toml::from_str(&content).ok()
}

fn try_native_port_forward(fwd: &PortForward) -> Result<(), String> {
    // Only support pod port-forwarding natively for now
    if fwd.r#type != "pod" {
        return Err("Native port-forward only supports pods".to_string());
    }
    let rt = Runtime::new().map_err(|e| e.to_string())?;
    let pod_name = &fwd.name;
    let ns = &fwd.namespace;
    let local_port = fwd.local_port;
    let remote_port = fwd.remote_port;
    rt.block_on(async move {
        let client = Client::try_default().await.map_err(|e| e.to_string())?;
        let pods: Api<Pod> = Api::namespaced(client, ns);
        // NOTE: Portforward is behind the ws feature, so this is a placeholder for real implementation
        // let mut pf = pods.portforward(pod_name, &[remote_port]).await.map_err(|e| e.to_string())?;
        // let (mut stream, _handle) = pf.take_stream(remote_port).map_err(|e| e.to_string())?;
        // For demo, just print what would happen
        println!("(Native) Would port-forward pod {}:{} -> localhost:{}", pod_name, remote_port, local_port);
        Ok(())
    })
}

fn spawn_kubectl_port_forward(fwd: &PortForward) {
    let kind = match fwd.r#type.as_str() {
        "pod" => "pod",
        "service" => "svc",
        _ => {
            eprintln!("Unknown type: {}", fwd.r#type);
            return;
        }
    };
    let target = format!("{}/{}", kind, fwd.name);
    let port_map = format!("{}:{}", fwd.local_port, fwd.remote_port);
    let mut cmd = ProcessCommand::new("kubectl");
    cmd.arg("port-forward")
        .arg(target)
        .arg(port_map)
        .arg("-n").arg(&fwd.namespace)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    match cmd.spawn() {
        Ok(mut child) => {
            println!("Spawned kubectl port-forward for {}", fwd.name);
            // Optionally: child.wait().unwrap();
        }
        Err(e) => {
            eprintln!("Failed to spawn kubectl: {}", e);
        }
    }
}

impl Plugin for ProxyPlugin {
    fn name(&self) -> &'static str {
        "k8s_port_forward"
    }

    fn subcommand(&self) -> Command {
        Command::new(self.name())
            .about("Port-forward as defined in config file (~/.cohandv/proxy/config/plugins.d/k8s_port_forward.conf)")
            .arg(
                Arg::new("name")
                    .long("name")
                    .value_name("NAME")
                    .help("Name of the port-forward config to use (from config file)")
                    .required(false)
            )
    }

    fn run(&self, matches: &ArgMatches) {
        env_logger::init();

        match load_config(self.name()) {
            Some(cfg) => {
                let name_filter = matches.get_one::<String>("name");
                let forwards: Vec<_> = match name_filter {
                    Some(name) => cfg.forward.into_iter().filter(|f| &f.name == name).collect(),
                    None => cfg.forward,
                };
                if forwards.is_empty() {
                    if let Some(name) = name_filter {
                        eprintln!("No port-forward config found with name: {}", name);
                    } else {
                        eprintln!("No port-forward configs found in config file");
                    }
                } else {
                    println!("Loaded k8s_port_forward config:");
                    for fwd in forwards {
                        println!(
                            "  {} {}:{} -> localhost:{}",
                            fwd.r#type, fwd.name, fwd.remote_port, fwd.local_port
                        );
                        // Try native port-forwarding (kube crate)
                        if let Err(e) = try_native_port_forward(&fwd) {
                            eprintln!("Native port-forward failed: {}. Falling back to kubectl...", e);
                            spawn_kubectl_port_forward(&fwd);
                        }
                    }
                }
            }
            None => {
                eprintln!("Could not load config file for k8s_port_forward");
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn create_plugin() -> Box<dyn Plugin> {
    Box::new(ProxyPlugin)
}

// Example config (save as ~/.cohandv/proxy/config/plugins.d/k8s_port_forward.conf):
/*
[[forward]]
name = "my-service"
namespace = "default"
type = "service"
local_port = 8080
remote_port = 80

[[forward]]
name = "my-pod"
namespace = "default"
type = "pod"
local_port = 9090
remote_port = 9000
*/

// --- Module scope ---
use clap::{Arg, ArgMatches, Command};
use plugin_api::Plugin;
// Removed unused log imports
use serde::Deserialize;
use std::fs;
use std::process::Command as ProcessCommand;
use std::process::Stdio;

#[derive(Debug, Deserialize)]
pub struct ForwardConfig {
    pub forward: Vec<PortForward>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PortForward {
    pub name: Option<String>,
    pub labels: Option<String>, // e.g. "app=nginx,version=v1"
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
labels = "app=nginx,version=v1"
namespace = "default"
type = "pod"
local_port = 9090
remote_port = 9000

[[forward]]
name = "my-pod"
namespace = "default"
type = "pod"
local_port = 3000
remote_port = 3000
"#
    }
}

fn load_config(plugin_name: &str) -> Option<ForwardConfig> {
    let config_path = plugin_api::plugin_config_path(plugin_name)?;
    let content = fs::read_to_string(config_path).ok()?;
    toml::from_str(&content).ok()
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

    let port_map = format!("{}:{}", fwd.local_port, fwd.remote_port);
    let mut cmd = ProcessCommand::new("kubectl");
    cmd.arg("port-forward");

    // Handle name vs labels
    match (&fwd.name, &fwd.labels) {
        (Some(name), None) => {
            let target = format!("{}/{}", kind, name);
            cmd.arg(target);
        }
        (_, Some(labels)) => {
            // First, list matching resources to show what we found
            let mut list_cmd = ProcessCommand::new("kubectl");
            list_cmd
                .arg("get")
                .arg(kind)
                .arg("-l")
                .arg(labels)
                .arg("-n")
                .arg(&fwd.namespace)
                .arg("--no-headers")
                .arg("-o")
                .arg("name");

            match list_cmd.output() {
                Ok(output) => {
                    let resources: Vec<&str> = std::str::from_utf8(&output.stdout)
                        .unwrap_or("")
                        .lines()
                        .filter(|line| !line.is_empty())
                        .collect();

                    if resources.is_empty() {
                        eprintln!("No {} found matching labels: {}", kind, labels);
                        return;
                    } else if resources.len() > 1 {
                        println!(
                            "Found {} {}(s) matching labels '{}': {}",
                            resources.len(),
                            kind,
                            labels,
                            resources.join(", ")
                        );
                        println!("Using the first one: {}", resources[0]);
                    } else {
                        println!(
                            "Found {} matching labels '{}': {}",
                            kind, labels, resources[0]
                        );
                    }

                    // Use the actual name of the first resource
                    cmd.arg(resources[0]);
                }
                Err(e) => {
                    eprintln!("Failed to list resources with labels {}: {}", labels, e);
                    return;
                }
            }
        }
        (None, None) => {
            eprintln!("Must specify either 'name' or 'labels' for port-forward config");
            return;
        }
    }

    cmd.arg(port_map)
        .arg("-n")
        .arg(&fwd.namespace)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    match cmd.spawn() {
        Ok(mut child) => {
            let target_desc = match (&fwd.name, &fwd.labels) {
                (Some(name), None) => name.clone(),
                (None, Some(labels)) => format!("labels:{}", labels),
                _ => "unknown".to_string(),
            };
            println!(
                "Spawned kubectl port-forward for {} (blocking, Ctrl-C will terminate)",
                target_desc
            );
            // Set up Ctrl-C handler to kill child
            let child_id = child.id();
            let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
            let r = running.clone();
            let _ = ctrlc::set_handler(move || {
                r.store(false, std::sync::atomic::Ordering::SeqCst);
                // Try to kill the child process
                #[cfg(unix)]
                unsafe {
                    libc::kill(child_id as i32, libc::SIGTERM);
                }
                #[cfg(windows)]
                {
                    let _ = ProcessCommand::new("taskkill")
                        .arg("/PID")
                        .arg(child_id.to_string())
                        .arg("/F")
                        .status();
                }
            });
            // Wait for child to exit
            let status = child.wait();
            running.store(false, std::sync::atomic::Ordering::SeqCst);
            match status {
                Ok(s) => println!("kubectl exited with status: {}", s),
                Err(e) => eprintln!("kubectl wait error: {}", e),
            }
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

    fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    fn description(&self) -> &'static str {
        "Kubernetes port forwarding with name and label support"
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
                    Some(name) => {
                        // Find exact name match first
                        let exact_matches: Vec<_> = cfg
                            .forward
                            .iter()
                            .filter(|f| f.name.as_ref() == Some(name))
                            .cloned()
                            .collect();

                        if !exact_matches.is_empty() {
                            exact_matches
                        } else {
                            // If no exact name match, try label substring match
                            cfg.forward
                                .into_iter()
                                .filter(|f| {
                                    f.labels
                                        .as_ref()
                                        .is_some_and(|labels| labels.contains(name))
                                })
                                .collect()
                        }
                    }
                    None => cfg.forward,
                };
                if forwards.is_empty() {
                    if let Some(name) = name_filter {
                        eprintln!("No port-forward config found with name: {}", name);
                    } else {
                        eprintln!("No port-forward configs found in config file");
                    }
                } else {
                    if forwards.len() > 1 && name_filter.is_some() {
                        println!("Found {} matching configurations:", forwards.len());
                        for fwd in &forwards {
                            let target_desc = match (&fwd.name, &fwd.labels) {
                                (Some(name), None) => name.clone(),
                                (None, Some(labels)) => format!("labels:{}", labels),
                                _ => "invalid-config".to_string(),
                            };
                            println!(
                                "  {} {}:{} -> localhost:{}",
                                fwd.r#type, target_desc, fwd.remote_port, fwd.local_port
                            );
                        }
                        println!("Using the first match only.\n");
                    }

                    // Only use the first forward to avoid conflicts
                    let fwd = &forwards[0];
                    let target_desc = match (&fwd.name, &fwd.labels) {
                        (Some(name), None) => name.clone(),
                        (None, Some(labels)) => format!("labels:{}", labels),
                        _ => "invalid-config".to_string(),
                    };

                    if forwards.len() == 1 || name_filter.is_none() {
                        println!("Starting port-forward:");
                    }
                    println!(
                        "  {} {}:{} -> localhost:{}",
                        fwd.r#type, target_desc, fwd.remote_port, fwd.local_port
                    );

                    spawn_kubectl_port_forward(fwd);
                }
            }
            None => {
                eprintln!("Could not load config file for k8s_port_forward");
            }
        }
    }
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
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
labels = "app=nginx,version=v1"
namespace = "default"
type = "pod"
local_port = 9090
remote_port = 9000

[[forward]]
name = "my-pod"
namespace = "default"
type = "pod"
local_port = 3000
remote_port = 3000
*/

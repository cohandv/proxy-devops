use clap::{Arg, ArgMatches, Command};
use plugin_api::Plugin;
use serde::Deserialize;
use std::fs;
use tokio::runtime::Runtime;
use anyhow::Result;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use kube::{Api, Client};
use k8s_openapi::api::core::v1::Pod;
use std::sync::Arc;
use chrono::Utc;

#[derive(Debug, Deserialize, Clone)]
pub struct K8sNativeConfig {
    pub namespace: String,
    pub pod_name: Option<String>,
    pub pod_selector: Option<String>, // label selector
    pub local_port: u16,
    pub remote_port: u16,
    pub protocol: Option<String>, // http, postgres, tcp (default)
}

impl Default for K8sNativeConfig {
    fn default() -> Self {
        Self {
            namespace: "default".to_string(),
            pod_name: None,
            pod_selector: None,
            local_port: 8080,
            remote_port: 80,
            protocol: Some("tcp".to_string()),
        }
    }
}

pub struct K8sNativePortForwardPlugin;

impl K8sNativePortForwardPlugin {
    pub fn sample_config() -> &'static str {
        r#"# Kubernetes Native Port Forward Configuration
namespace = "default"
pod_name = "my-pod"  # Either use pod_name OR pod_selector
# pod_selector = "app=nginx,version=v1"  # Label selector alternative
local_port = 8080
remote_port = 80
protocol = "http"  # Options: tcp, http, postgres

# Example configurations:
# For HTTP service:
# protocol = "http"
# local_port = 8080
# remote_port = 80

# For PostgreSQL database:
# protocol = "postgres"
# local_port = 5432
# remote_port = 5432

# For generic TCP (no message decoding):
# protocol = "tcp"
"#
    }
}

#[derive(Debug, Clone)]
pub enum Protocol {
    Tcp,
    Http,
    Postgres,
}

impl From<&str> for Protocol {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "http" => Protocol::Http,
            "postgres" | "postgresql" => Protocol::Postgres,
            _ => Protocol::Tcp,
        }
    }
}

fn load_config(plugin_name: &str) -> Result<K8sNativeConfig> {
    match plugin_api::plugin_config_path(plugin_name) {
        Some(config_path) => {
            if config_path.exists() {
                let content = fs::read_to_string(config_path)?;
                let config: K8sNativeConfig = toml::from_str(&content)?;
                Ok(config)
            } else {
                println!("⚠️  Config file not found, using defaults.");
                println!("💡 Create config at: {}", config_path.display());
                println!("📝 Sample config:\n{}", K8sNativePortForwardPlugin::sample_config());
                Ok(K8sNativeConfig::default())
            }
        }
        None => {
            println!("⚠️  Could not determine config path, using defaults.");
            Ok(K8sNativeConfig::default())
        }
    }
}

fn log_message(direction: &str, protocol: &Protocol, data: &[u8]) {
    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f UTC").to_string();

    match protocol {
        Protocol::Http => log_http_message(direction, data, &timestamp),
        Protocol::Postgres => log_postgres_message(direction, data, &timestamp),
        Protocol::Tcp => log_tcp_message(direction, data, &timestamp),
    }
}

fn log_http_message(direction: &str, data: &[u8], timestamp: &str) {
    if let Ok(text) = std::str::from_utf8(data) {
        // Try to parse as HTTP
        if text.starts_with("GET ") || text.starts_with("POST ") ||
           text.starts_with("PUT ") || text.starts_with("DELETE ") ||
           text.starts_with("HTTP/") {
            println!("🌐 [{}] {} HTTP Message:", timestamp, direction);

            // Split headers and body
            if let Some(header_end) = text.find("\r\n\r\n") {
                let headers = &text[..header_end];
                let body = &text[header_end + 4..];

                println!("   Headers:");
                for line in headers.lines() {
                    println!("     {}", line);
                }

                if !body.is_empty() {
                    println!("   Body:");
                    println!("     {}", body);
                }
            } else {
                println!("   {}", text);
            }
        } else {
            log_tcp_message(direction, data, timestamp);
        }
    } else {
        log_tcp_message(direction, data, timestamp);
    }
}

fn log_postgres_message(direction: &str, data: &[u8], timestamp: &str) {
    if data.is_empty() {
        return;
    }

    println!("🐘 [{}] {} PostgreSQL Message:", timestamp, direction);

    // Basic PostgreSQL protocol parsing
    if data.len() >= 5 {
        let msg_type = data[0] as char;
        let length = u32::from_be_bytes([data[1], data[2], data[3], data[4]]);

        match msg_type {
            'Q' => {
                if let Ok(query) = std::str::from_utf8(&data[5..]) {
                    println!("   Query: {}", query.trim_end_matches('\0'));
                }
            }
            'P' => println!("   Parse message (length: {})", length),
            'B' => println!("   Bind message (length: {})", length),
            'E' => println!("   Execute message (length: {})", length),
            'S' => println!("   Sync message"),
            'X' => println!("   Terminate message"),
            'T' => println!("   Row Description (length: {})", length),
            'D' => println!("   Data Row (length: {})", length),
            'C' => {
                if let Ok(command) = std::str::from_utf8(&data[5..]) {
                    println!("   Command Complete: {}", command.trim_end_matches('\0'));
                }
            }
            'Z' => println!("   Ready for Query"),
            'R' => println!("   Authentication Response (length: {})", length),
            _ => {
                println!("   Unknown message type '{}' (length: {})", msg_type, length);
                println!("   Raw data: {}", hex::encode(&data[..std::cmp::min(50, data.len())]));
            }
        }
    } else {
        log_tcp_message(direction, data, timestamp);
    }
}

fn log_tcp_message(direction: &str, data: &[u8], timestamp: &str) {
    println!("🔌 [{}] {} TCP Message ({} bytes):", timestamp, direction, data.len());

    // Show first 100 bytes as hex and try to show as text if printable
    let preview_len = std::cmp::min(100, data.len());
    let preview = &data[..preview_len];

    println!("   Hex: {}", hex::encode(preview));

    if let Ok(text) = std::str::from_utf8(preview) {
        if text.chars().all(|c| c.is_ascii() && (c.is_ascii_graphic() || c.is_ascii_whitespace())) {
            println!("   Text: {}", text.replace('\n', "\\n").replace('\r', "\\r"));
        }
    }

    if data.len() > preview_len {
        println!("   ... ({} more bytes)", data.len() - preview_len);
    }
}

async fn find_pod_by_selector(client: &Client, namespace: &str, selector: &str) -> Result<String> {
    let pods: Api<Pod> = Api::namespaced(client.clone(), namespace);

    let lp = kube::api::ListParams::default().labels(selector);
    let pod_list = pods.list(&lp).await?;

    if pod_list.items.is_empty() {
        return Err(anyhow::anyhow!("No pods found matching selector: {}", selector));
    }

    if pod_list.items.len() > 1 {
        println!("Found {} pods matching selector '{}', using the first one:",
                 pod_list.items.len(), selector);
        for pod in &pod_list.items {
            if let Some(name) = &pod.metadata.name {
                println!("  - {}", name);
            }
        }
    }

    let pod_name = pod_list.items[0].metadata.name.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Pod has no name"))?;

    Ok(pod_name.clone())
}

// Handle connection using native Kubernetes API
async fn handle_native_connection(
    mut client_stream: TcpStream,
    k8s_client: Client,
    namespace: String,
    pod_name: String,
    remote_port: u16,
    protocol: Protocol,
) -> Result<()> {
    use kube::api::AttachParams;

    println!("🔗 Establishing connection to pod via Kubernetes API");

    let pods: Api<Pod> = Api::namespaced(k8s_client, &namespace);

    // Use Kubernetes exec API with socat to create a bidirectional stream
    let attach_params = AttachParams {
        container: None,
        tty: false,
        stdin: true,
        stdout: true,
        stderr: true,
        max_stdin_buf_size: None,
        max_stdout_buf_size: None,
        max_stderr_buf_size: None,
    };

    // Use bash with /dev/tcp for bidirectional TCP connection
    // This works in most containers that have bash without additional tools
    // The script:
    // 1. Opens a bidirectional connection to localhost:port via file descriptor 3
    // 2. Starts background process to copy from FD 3 to stdout
    // 3. Copies from stdin to FD 3 in foreground
    // 4. When stdin closes, kills the background job and closes FD 3
    let exec_command = vec![
        "bash".to_string(),
        "-c".to_string(),
        format!(
            "exec 3<>/dev/tcp/localhost/{}; (cat <&3 &); cat >&3; kill %1 2>/dev/null; exec 3>&-",
            remote_port
        ),
    ];

    let mut attached = pods
        .exec(&pod_name, exec_command, &attach_params)
        .await?;

    println!("✅ Connected to pod via native Kubernetes API");

    let (mut client_read, mut client_write) = client_stream.split();

    let protocol_clone = protocol.clone();
    let protocol_clone2 = protocol.clone();

    // Get stdin/stdout from the attached process
    let mut pod_stdin = attached.stdin().ok_or_else(|| anyhow::anyhow!("No stdin"))?;
    let mut pod_stdout = attached.stdout().ok_or_else(|| anyhow::anyhow!("No stdout"))?;

    // Handle client -> pod
    let client_to_pod = async move {
        let mut buffer = vec![0u8; 8192];
        loop {
            match client_read.read(&mut buffer).await {
                Ok(0) => break, // Connection closed
                Ok(n) => {
                    let data = &buffer[..n];
                    log_message("→ REQUEST", &protocol_clone, data);

                    if let Err(e) = pod_stdin.write_all(data).await {
                        eprintln!("Error writing to pod: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("Error reading from client: {}", e);
                    break;
                }
            }
        }
    };

    // Handle pod -> client
    let pod_to_client = async move {
        let mut buffer = vec![0u8; 8192];

        loop {
            match pod_stdout.read(&mut buffer).await {
                Ok(0) => break, // Connection closed
                Ok(n) => {
                    let data = &buffer[..n];
                    log_message("← RESPONSE", &protocol_clone2, data);

                    if let Err(e) = client_write.write_all(data).await {
                        eprintln!("Error writing to client: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("Error reading from pod: {}", e);
                    break;
                }
            }
        }
    };

    // Run both directions concurrently
    tokio::select! {
        _ = client_to_pod => {},
        _ = pod_to_client => {},
    }

    println!("🔌 Connection closed");
    Ok(())
}

async fn start_port_forward(config: K8sNativeConfig, protocol_override: Option<String>) -> Result<()> {
    let protocol = Protocol::from(
        protocol_override.as_deref()
            .or(config.protocol.as_deref())
            .unwrap_or("tcp")
    );

    println!("🚀 Starting Kubernetes Native Port Forward with Message Logging");
    println!("📡 Namespace: {}", config.namespace);
    println!("🎯 Protocol: {:?}", protocol);
    println!("🔌 Local port: {}", config.local_port);
    println!("🎯 Remote port (kubectl): {}", config.remote_port);
    println!("🎯 Remote port (pod): {}", config.remote_port);

    // Create Kubernetes client
    let k8s_client = Client::try_default().await?;

    // Determine pod name
    let pod_name = if let Some(name) = config.pod_name {
        println!("📦 Pod name: {}", name);
        name
    } else if let Some(selector) = config.pod_selector {
        println!("🏷️  Pod selector: {}", selector);
        let name = find_pod_by_selector(&k8s_client, &config.namespace, &selector).await?;
        println!("📦 Selected pod: {}", name);
        name
    } else {
        return Err(anyhow::anyhow!("Must specify either pod_name or pod_selector"));
    };

    println!("📝 Strategy: Using native Kubernetes API (exec + socat)");
    println!("   This uses the Kubernetes API SDK directly without kubectl\n");

    // Set up Ctrl+C handler
    let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, std::sync::atomic::Ordering::SeqCst);
        println!("\n👋 Shutting down...");
        std::process::exit(0);
    })?;

    println!("🎧 Listening on 127.0.0.1:{}", config.local_port);
    println!("🔄 Forwarding to pod {}:{} via native K8s API", pod_name, config.remote_port);
    println!("⚡ Ready to log {} traffic", match protocol {
        Protocol::Http => "HTTP",
        Protocol::Postgres => "PostgreSQL",
        Protocol::Tcp => "TCP",
    });

    println!();

    // Start listening for connections
    let listener = TcpListener::bind(format!("127.0.0.1:{}", config.local_port)).await?;

    while running.load(std::sync::atomic::Ordering::SeqCst) {
        match listener.accept().await {
            Ok((client_stream, client_addr)) => {
                println!("📞 New connection from {}", client_addr);

                let pod_name_clone = pod_name.clone();
                let namespace_clone = config.namespace.clone();
                let protocol_clone = protocol.clone();
                let client_clone = k8s_client.clone();
                let remote_port = config.remote_port;

                tokio::spawn(async move {
                    if let Err(e) = handle_native_connection(
                        client_stream,
                        client_clone,
                        namespace_clone,
                        pod_name_clone,
                        remote_port,
                        protocol_clone,
                    ).await {
                        eprintln!("❌ Connection error: {}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("❌ Failed to accept connection: {}", e);
            }
        }
    }

    Ok(())
}

impl Plugin for K8sNativePortForwardPlugin {
    fn name(&self) -> &'static str {
        "k8s_native_port_forward"
    }

    fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    fn description(&self) -> &'static str {
        "Native Kubernetes port forwarding with protocol-aware message logging"
    }

    fn subcommand(&self) -> Command {
        Command::new(self.name())
            .about("Native Kubernetes port forwarding with message logging")
            .arg(
                Arg::new("pod")
                    .long("pod")
                    .short('p')
                    .value_name("POD_NAME")
                    .help("Override pod name from config file"),
            )
            .arg(
                Arg::new("selector")
                    .long("selector")
                    .short('s')
                    .value_name("SELECTOR")
                    .help("Override pod selector from config file (e.g., 'app=nginx,version=v1')"),
            )
            .arg(
                Arg::new("namespace")
                    .long("namespace")
                    .short('n')
                    .value_name("NAMESPACE")
                    .help("Override namespace from config file"),
            )
            .arg(
                Arg::new("local-port")
                    .long("local-port")
                    .short('l')
                    .value_name("PORT")
                    .help("Override local port from config file")
                    .value_parser(clap::value_parser!(u16)),
            )
            .arg(
                Arg::new("remote-port")
                    .long("remote-port")
                    .short('r')
                    .value_name("PORT")
                    .help("Override remote port from config file")
                    .value_parser(clap::value_parser!(u16)),
            )
            .arg(
                Arg::new("protocol")
                    .long("protocol")
                    .value_name("PROTOCOL")
                    .help("Protocol for message decoding: tcp, http, postgres")
                    .value_parser(["tcp", "http", "postgres"]),
            )
    }

    fn run(&self, matches: &ArgMatches) {
        let rt = Runtime::new().expect("Failed to create Tokio runtime");

        rt.block_on(async {
            let mut config = match load_config(self.name()) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!("❌ Failed to load config: {}", e);
                    std::process::exit(1);
                }
            };

            // Override config with command line arguments
            if let Some(pod) = matches.get_one::<String>("pod") {
                if pod.is_empty() {
                    eprintln!("❌ Pod name cannot be empty");
                    std::process::exit(1);
                }
                config.pod_name = Some(pod.clone());
                config.pod_selector = None; // Clear selector if pod name is specified
            }

            if let Some(selector) = matches.get_one::<String>("selector") {
                if selector.is_empty() {
                    eprintln!("❌ Pod selector cannot be empty");
                    std::process::exit(1);
                }
                config.pod_selector = Some(selector.clone());
                config.pod_name = None; // Clear pod name if selector is specified
            }

            if let Some(namespace) = matches.get_one::<String>("namespace") {
                if namespace.is_empty() {
                    eprintln!("❌ Namespace cannot be empty");
                    std::process::exit(1);
                }
                config.namespace = namespace.clone();
            }

            if let Some(local_port) = matches.get_one::<u16>("local-port") {
                config.local_port = *local_port;
            }

            if let Some(remote_port) = matches.get_one::<u16>("remote-port") {
                config.remote_port = *remote_port;
            }

            // Validate that either pod name or selector is provided
            if config.pod_name.is_none() && config.pod_selector.is_none() {
                eprintln!("❌ Must specify either --pod or --selector (or configure in config file)");
                eprintln!("💡 Example: proxy k8s_native_port_forward --pod my-pod --local-port 8080 --remote-port 80");
                eprintln!("💡 Example: proxy k8s_native_port_forward --selector app=nginx --local-port 8080 --remote-port 80");
                std::process::exit(1);
            }

            let protocol_override = matches.get_one::<String>("protocol").cloned();

            if let Err(e) = start_port_forward(config, protocol_override).await {
                eprintln!("❌ Port forward error: {}", e);
                std::process::exit(1);
            }
        });
    }
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn create_plugin() -> Box<dyn Plugin> {
    Box::new(K8sNativePortForwardPlugin)
}

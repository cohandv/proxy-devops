use clap::{Arg, ArgMatches, Command};
use futures::StreamExt;
use plugin_api::Plugin;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use tokio::runtime::Runtime;
// Crossterm imports for future terminal enhancements if needed

#[derive(Debug, Deserialize, Clone)]
pub struct OllamaConfig {
    pub url: String,
    pub model: String,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub top_k: Option<i32>,
    pub system_prompt: Option<String>,
    pub stream: Option<bool>,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:11434".to_string(),
            model: "llama3.1:8b".to_string(),
            temperature: Some(0.7),
            top_p: Some(0.9),
            top_k: Some(40),
            system_prompt: Some("You are a helpful AI assistant.".to_string()),
            stream: Some(true),
        }
    }
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<ChatOptions>,
}

#[derive(Debug, Serialize)]
struct ChatOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    message: Option<Message>,
    done: bool,
}

pub struct OllamaChatPlugin;

impl OllamaChatPlugin {
    pub fn sample_config() -> &'static str {
        r#"# Ollama Chat Configuration
url = "http://localhost:11434"
model = "llama3.1:8b"
temperature = 0.7
top_p = 0.9
top_k = 40
system_prompt = "You are a helpful AI assistant specialized in software development and technical support."
stream = true

# Alternative configurations:
# For Code Generation:
# model = "codellama:13b"
# system_prompt = "You are an expert programmer. Provide clean, well-commented code."

# For General Chat:
# model = "llama3.1:70b"
# temperature = 0.8
# system_prompt = "You are a friendly and knowledgeable assistant."
"#
    }
}

fn load_config(plugin_name: &str) -> anyhow::Result<OllamaConfig> {
    match plugin_api::plugin_config_path(plugin_name) {
        Some(config_path) => {
            if config_path.exists() {
                let content = fs::read_to_string(config_path)?;
                let config: OllamaConfig = toml::from_str(&content)?;
                Ok(config)
            } else {
                println!("⚠️  Config file not found, using defaults.");
                println!("💡 Create config at: {}", config_path.display());
                println!("📝 Sample config:\n{}", OllamaChatPlugin::sample_config());
                Ok(OllamaConfig::default())
            }
        }
        None => {
            println!("⚠️  Could not determine config path, using defaults.");
            Ok(OllamaConfig::default())
        }
    }
}

async fn send_chat_message(
    client: &Client,
    config: &OllamaConfig,
    messages: &[Message],
) -> anyhow::Result<()> {
    let options = ChatOptions {
        temperature: config.temperature,
        top_p: config.top_p,
        top_k: config.top_k,
    };

    let request = ChatRequest {
        model: config.model.clone(),
        messages: messages.to_vec(),
        stream: config.stream.unwrap_or(true),
        options: Some(options),
    };

    let response = client
        .post(format!("{}/api/chat", config.url))
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("Ollama API error: {}", error_text));
    }

    print!("🤖 ");
    io::stdout().flush()?;

    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let text = String::from_utf8_lossy(&chunk);

        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<ChatResponse>(line) {
                Ok(chat_response) => {
                    if let Some(message) = chat_response.message {
                        print!("{}", message.content);
                        io::stdout().flush()?;
                    }
                    if chat_response.done {
                        println!("\n");
                        return Ok(());
                    }
                }
                Err(_) => {
                    // Skip invalid JSON lines
                    continue;
                }
            }
        }
    }

    println!("\n");
    Ok(())
}

async fn run_chat_loop(config: OllamaConfig) -> anyhow::Result<()> {
    let client = Client::new();
    let mut messages = Vec::new();

    // Add system prompt if configured
    if let Some(system_prompt) = &config.system_prompt {
        messages.push(Message {
            role: "system".to_string(),
            content: system_prompt.clone(),
        });
    }

    println!("🚀 Ollama Chat Interface");
    println!("📡 Connected to: {}", config.url);
    println!("🤖 Using model: {}", config.model);
    println!("💬 Type your messages (Ctrl+C to exit, 'clear' to reset conversation)\n");

    // Set up Ctrl+C handler
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, std::sync::atomic::Ordering::SeqCst);
        println!("\n👋 Goodbye!");
        std::process::exit(0);
    })?;

    loop {
        // Check if we should continue
        if !running.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }

        print!("🧑 ");
        io::stdout().flush()?;

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                let input = input.trim();

                if input.is_empty() {
                    continue;
                }

                if input.eq_ignore_ascii_case("clear") {
                    messages.clear();
                    // Re-add system prompt if configured
                    if let Some(system_prompt) = &config.system_prompt {
                        messages.push(Message {
                            role: "system".to_string(),
                            content: system_prompt.clone(),
                        });
                    }
                    println!("🧹 Conversation cleared!\n");
                    continue;
                }

                if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
                    break;
                }

                // Add user message
                messages.push(Message {
                    role: "user".to_string(),
                    content: input.to_string(),
                });

                // Send to Ollama and stream response
                match send_chat_message(&client, &config, &messages).await {
                    Ok(_) => {
                        // Add assistant response placeholder (we don't capture the streamed response)
                        // In a real implementation, you'd capture the full response
                        println!();
                    }
                    Err(e) => {
                        println!("❌ Error: {}\n", e);
                        // Remove the failed user message
                        messages.pop();
                    }
                }
            }
            Err(e) => {
                println!("❌ Input error: {}", e);
                break;
            }
        }
    }

    println!("👋 Chat session ended.");
    Ok(())
}

impl Plugin for OllamaChatPlugin {
    fn name(&self) -> &'static str {
        "ollama_chat"
    }

    fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    fn description(&self) -> &'static str {
        "Interactive streaming chat interface for Ollama"
    }

    fn subcommand(&self) -> Command {
        Command::new(self.name())
            .about("Interactive chat with Ollama models")
            .arg(
                Arg::new("model")
                    .long("model")
                    .short('m')
                    .value_name("MODEL")
                    .help("Override the model from config file"),
            )
            .arg(
                Arg::new("url")
                    .long("url")
                    .short('u')
                    .value_name("URL")
                    .help("Override the Ollama URL from config file"),
            )
            .arg(
                Arg::new("temperature")
                    .long("temperature")
                    .short('t')
                    .value_name("TEMP")
                    .help("Set temperature (0.0-1.0)")
                    .value_parser(clap::value_parser!(f32)),
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
            if let Some(model) = matches.get_one::<String>("model") {
                config.model = model.clone();
            }

            if let Some(url) = matches.get_one::<String>("url") {
                config.url = url.clone();
            }

            if let Some(temperature) = matches.get_one::<f32>("temperature") {
                config.temperature = Some(*temperature);
            }

            if let Err(e) = run_chat_loop(config).await {
                eprintln!("❌ Chat error: {}", e);
                std::process::exit(1);
            }
        });
    }
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn create_plugin() -> Box<dyn Plugin> {
    Box::new(OllamaChatPlugin)
}

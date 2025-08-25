use plugin_api::Plugin;

pub struct ProxyPlugin;

impl Plugin for ProxyPlugin {
    fn name(&self) -> &'static str {
        "ProxyPlugin"
    }
    fn run(&self) {
        use clap::{Arg, Command};
        use log::{debug, info, warn, error};
        use std::process;

        // Initialize logger - set RUST_LOG environment variable to control level
        env_logger::init();

        debug!("Starting application");

        let matches = Command::new("proxy")
            .version("0.1.0")
            .about("A command line proxy tool")
            .arg(
                Arg::new("port")
                    .short('p')
                    .long("port")
                    .value_name("PORT")
                    .help("Sets the port to listen on")
                    .default_value("8080")
            )
            .arg(
                Arg::new("target")
                    .short('t')
                    .long("target")
                    .value_name("TARGET")
                    .help("Sets the target URL to proxy to")
                    .required(true)
            )
            .arg(
                Arg::new("verbose")
                    .short('v')
                    .long("verbose")
                    .help("Enable verbose output")
                    .action(clap::ArgAction::SetTrue)
            )
            .get_matches();

        let port = matches.get_one::<String>("port").unwrap();
        let target = matches.get_one::<String>("target").unwrap();
        let verbose = matches.get_flag("verbose");

        debug!("Parsed command line arguments");
        debug!("Port: {}", port);
        debug!("Target: {}", target);
        debug!("Verbose: {}", verbose);

        if verbose {
            info!("Starting proxy on port {} -> {}", port, target);
        }

        // Example of different log levels for debugging
        info!("Proxy CLI configured successfully");
        warn!("This is a warning message for testing");
        error!("This is an error message for testing");

        println!("Proxy CLI configured:");
        println!("  Port: {}", port);
        println!("  Target: {}", target);
        println!("  Verbose: {}", verbose);

        debug!("About to exit application");

        // For now, just exit successfully
        process::exit(0);
    }
}

#[no_mangle]
pub extern "C" fn create_plugin() -> Box<dyn Plugin> {
    Box::new(ProxyPlugin)
}

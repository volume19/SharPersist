//! SharPersist CLI binary

use sharpersist::CliArgs;

#[cfg(target_os = "windows")]
use sharpersist::{core::config::Technique, techniques};

fn main() {
    env_logger::init();

    let args = CliArgs::parse_args();

    match args.into_config() {
        Ok(config) => {
            println!("[*] Configuration validated successfully");
            println!("[*] Technique: {}", config.technique);
            println!("[*] Method: {}", config.method);

            #[cfg(target_os = "windows")]
            {
                let result = match config.technique {
                    Technique::Registry => techniques::registry::execute(&config),
                    _ => {
                        println!("\n[!] Technique not yet implemented - placeholder only");
                        eprintln!(
                            "[-] ERROR: Technique '{}' is not yet implemented",
                            config.technique
                        );
                        std::process::exit(1);
                    }
                };

                if let Err(e) = result {
                    eprintln!("[-] ERROR: {}", e);
                    std::process::exit(1);
                }
            }

            #[cfg(not(target_os = "windows"))]
            {
                eprintln!("[-] ERROR: Windows-only techniques not supported on this platform");
                eprintln!("[-] Technique '{}' requires Windows", config.technique);
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("[-] ERROR: {}", e);
            std::process::exit(1);
        }
    }
}

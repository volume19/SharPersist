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
                    Technique::Service => techniques::service::execute(&config),
                    Technique::KeePass => techniques::keepass::execute(&config),
                    Technique::StartupFolder => techniques::startup_folder::execute(&config),
                    Technique::TortoiseSVN => techniques::tortoisesvn::execute(&config),
                    Technique::SchTask | Technique::SchTaskBackdoor => {
                        println!("\n[!] Scheduled Task techniques require COM interop");
                        eprintln!(
                            "[-] ERROR: Technique '{}' is not yet implemented in Rust port",
                            config.technique
                        );
                        eprintln!(
                            "[*] INFO: This technique requires complex COM automation which is planned for future implementation"
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

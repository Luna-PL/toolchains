use luna_compiler::{DiscoveryConfig, discover};
use std::env;
use std::path::PathBuf;

fn main() {
    let mut arguments = env::args_os().skip(1);
    let Some(command) = arguments.next() else {
        println!("LunaToolchain 0.1.1");
        return;
    };
    if command != "compiler" {
        eprintln!("unknown command; expected `compiler`");
        std::process::exit(2);
    }

    let mut config = DiscoveryConfig::from_environment();
    while let Some(argument) = arguments.next() {
        if argument == "--luna" {
            let Some(path) = arguments.next() else {
                eprintln!("--luna requires an executable path");
                std::process::exit(2);
            };
            config.explicit = Some(PathBuf::from(path));
        } else {
            eprintln!("unknown compiler option: {}", argument.to_string_lossy());
            std::process::exit(2);
        }
    }

    match discover(&config) {
        Ok(compiler) => {
            println!("path={}", compiler.executable.display());
            println!("source={:?}", compiler.source);
            println!("language={}", compiler.identity.language_version);
            println!("commit={}", compiler.identity.compiler_commit);
            println!("target={}", compiler.identity.build_target);
            println!(
                "diagnostic_protocol={}",
                compiler.identity.diagnostic_protocol_version
            );
            println!("capabilities={}", compiler.identity.capabilities.join(","));
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

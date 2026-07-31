use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

enum Mode {
    Print,
    Write,
    Check,
}

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let mut mode = Mode::Print;
    let mut paths = Vec::new();
    for argument in env::args_os().skip(1) {
        if argument == "--write" {
            if matches!(mode, Mode::Check) {
                eprintln!("--write and --check cannot be used together");
                return 2;
            }
            mode = Mode::Write;
        } else if argument == "--check" {
            if matches!(mode, Mode::Write) {
                eprintln!("--write and --check cannot be used together");
                return 2;
            }
            mode = Mode::Check;
        } else if argument == "--help" || argument == "-h" {
            print_usage();
            return 0;
        } else if argument != "-" && argument.to_string_lossy().starts_with('-') {
            eprintln!("unknown option: {}", argument.to_string_lossy());
            return 2;
        } else {
            paths.push(PathBuf::from(argument));
        }
    }
    if paths.is_empty() || matches!(mode, Mode::Print) && paths.len() != 1 {
        print_usage();
        return 2;
    }

    let mut changed = false;
    for path in paths {
        if path.as_os_str() == "-" && !matches!(mode, Mode::Print) {
            eprintln!("stdin is supported only in print mode");
            return 2;
        }
        let source = match read_source(&path) {
            Ok(source) => source,
            Err(error) => {
                eprintln!("{}: {error}", path.display());
                return 2;
            }
        };
        let formatted = match luna_fmt::format_source(&source) {
            Ok(formatted) => formatted,
            Err(error) => {
                eprintln!("{}: {error}", path.display());
                return 2;
            }
        };
        match mode {
            Mode::Print => print!("{formatted}"),
            Mode::Write if formatted != source => {
                if let Err(error) = fs::write(&path, formatted) {
                    eprintln!("{}: {error}", path.display());
                    return 2;
                }
            }
            Mode::Check if formatted != source => {
                eprintln!("would reformat {}", path.display());
                changed = true;
            }
            Mode::Write | Mode::Check => {}
        }
    }
    i32::from(changed)
}

fn print_usage() {
    eprintln!(
        "Usage:\n  luna-fmt <file|->\n  luna-fmt --write <file>...\n  luna-fmt --check <file>..."
    );
}

fn read_source(path: &PathBuf) -> io::Result<String> {
    if path.as_os_str() == "-" {
        let mut source = String::new();
        io::stdin().read_to_string(&mut source)?;
        Ok(source)
    } else {
        fs::read_to_string(path)
    }
}

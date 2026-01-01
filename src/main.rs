use std::env;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::io::Write;

fn encode(source: &str) -> String {
    let mut output = String::from("fn main() {\n");

    for (line_no, line) in source.lines().enumerate() {
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        // print(...)
        if line.starts_with("print(") && line.ends_with(")--") {
            let inner = &line[6..line.len() - 3];

            if inner.starts_with('"') {
                output.push_str(&format!(
                    "    println!({});\n",
                    inner
                ));
            } else {
                output.push_str(&format!(
                    "    println!(\"{{}}\", {});\n",
                    inner
                ));
            }
        }

        // var x = value--
        else if line.starts_with("var ") && line.ends_with("--") {
            let stmt = &line[4..line.len() - 2];
            output.push_str(&format!("    let {};\n", stmt));
        }

        // unknown syntax
        else {
            panic!("Syntax error on line {}: {}", line_no + 1, line);
        }
    }

    output.push_str("}\n");
    output
}


fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: forge <file.fc1>");
        std::process::exit(1);
    }

    let path = Path::new(&args[1]);

    if path.extension().and_then(|s| s.to_str()) != Some("fc1") {
        eprintln!("Error: expected a .fc1 file");
        std::process::exit(1);
    }

    let source = fs::read_to_string(path)
        .expect("Failed to read .fc1 file");

    let rust_code = encode(&source);

    // Compile Rust code from stdin
    let mut rustc = Command::new("rustc")
        .arg("-")
        .arg("-o")
        .arg("forge_tmp_bin")
        .stdin(Stdio::piped())
        .spawn()
        .expect("Failed to start rustc");

    rustc
        .stdin
        .as_mut()
        .unwrap()
        .write_all(rust_code.as_bytes())
        .expect("Failed to write Rust code to rustc");

    let status = rustc.wait().expect("Failed to wait on rustc");

    if !status.success() {
        eprintln!("Compilation failed");
        std::process::exit(1);
    }

    // Run the compiled binary
    Command::new("./forge_tmp_bin")
        .status()
        .expect("Failed to run program");

    // Cleanup (optional but clean)
    let _ = fs::remove_file("forge_tmp_bin");
}

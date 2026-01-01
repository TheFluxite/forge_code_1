use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

/// Simple pseudo-random number generator
fn simple_rand(min: i32, max: i32) -> i32 {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
    min + (nanos as i32 % (max - min + 1))
}

/// Transpiles Forge Code 1 (.fc1) source into Rust code
fn encode(source: &str) -> String {
    let mut output = String::from("fn main() {\n");
    output.push_str("use std::io::{self, Write};\n");

    let mut indent_level = 1;

    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }

        let indent = "    ".repeat(indent_level);

        // print(...)
        if line.starts_with("print(") && line.ends_with(")--") {
            let inner = &line[6..line.len() - 3];
            if inner.starts_with('"') {
                output.push_str(&format!("{}println!({});\n", indent, inner));
            } else {
                output.push_str(&format!("{}println!(\"{{}}\", {});\n", indent, inner));
            }
        }
        // var ...
        else if line.starts_with("var ") && line.ends_with("--") {
            let stmt = &line[4..line.len() - 2];

            // Handle input
            if stmt.contains("= input(") {
                let parts: Vec<&str> = stmt.split("= input(").collect();
                let var_name = parts[0].trim();
                let prompt = parts[1].trim_end_matches(')').trim();
                output.push_str(&format!(
                    "{}print({}); io::stdout().flush().unwrap(); let mut {} = String::new(); io::stdin().read_line(&mut {}).unwrap(); {} = {}.trim().to_string();\n",
                    indent, prompt, var_name, var_name, var_name, var_name
                ));
            } else {
                output.push_str(&format!("{}let {};\n", indent, stmt));
            }
        }
        // if condition
        else if line.starts_with("if ") && line.ends_with("{") {
            let condition = &line[3..line.len()-1].trim();
            output.push_str(&format!("{}if {} {{\n", indent, condition));
            indent_level += 1;
        }
        // else
        else if line.starts_with("else") && line.ends_with("{") {
            indent_level -= 1;
            output.push_str(&format!("{}else {{\n", "    ".repeat(indent_level)));
            indent_level += 1;
        }
        // while
        else if line.starts_with("while ") && line.ends_with("{") {
            let condition = &line[6..line.len()-1].trim();
            output.push_str(&format!("{}while {} {{\n", indent, condition));
            indent_level += 1;
        }
        // closing brace
        else if line == "}" {
            indent_level -= 1;
            output.push_str(&format!("{}}}\n", "    ".repeat(indent_level)));
        }
        // replace rand(min,max) with simple_rand(min,max)
        else if line.contains("rand(") {
            let new_line = line.replace("rand(", "simple_rand(");
            output.push_str(&format!("{}{}\n", indent, new_line));
        }
        // other expressions (math, etc.)
        else {
            output.push_str(&format!("{}{}\n", indent, line));
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

    // Compile Rust code in-memory
    let mut rustc = Command::new("rustc")
        .arg("-")
        .arg("-o")
        .arg("forge_tmp_bin")
        .stdin(Stdio::piped())
        .spawn()
        .expect("Failed to start rustc");

    rustc.stdin.as_mut().unwrap()
        .write_all(rust_code.as_bytes())
        .expect("Failed to write Rust code to rustc");

    let status = rustc.wait().expect("Failed to wait on rustc");

    if !status.success() {
        eprintln!("Compilation failed");
        std::process::exit(1);
    }

    // Run compiled binary
    Command::new("./forge_tmp_bin")
        .status()
        .expect("Failed to run program");

    // Clean up
    let _ = fs::remove_file("forge_tmp_bin");
}

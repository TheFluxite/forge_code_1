use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;
use std::io::Write;

/// Transpiles Forge Code 1 (.fc1) source into Rust code
fn encode(source: &str) -> String {
    let mut output = String::from("fn main() {\n");
    output.push_str("use std::io::{self, Write};\n");
    output.push_str("use std::time::{SystemTime, UNIX_EPOCH};\n\n");

    // Include simple_rand function INSIDE generated code
    output.push_str("fn simple_rand(min: i32, max: i32) -> i32 {\n");
    output.push_str("    use std::collections::hash_map::RandomState;\n");
    output.push_str("    use std::hash::{BuildHasher, Hasher};\n");
    output.push_str("    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();\n");
    output.push_str("    let state = RandomState::new();\n");
    output.push_str("    let mut hasher = state.build_hasher();\n");
    output.push_str("    hasher.write_u128(nanos);\n");
    output.push_str("    let hash = hasher.finish();\n");
    output.push_str("    min + ((hash as i32).abs() % (max - min + 1))\n");
    output.push_str("}\n\n");

    let mut indent_level = 1;
    let mut in_block_comment = false;

    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }

        // Handle block comments that can span lines: /* ... */
        if in_block_comment {
            if line.contains("*/") {
                in_block_comment = false;
            }
            continue;
        }

        // Full-line block comment start
        if line.starts_with("/*") {
            if !line.contains("*/") {
                in_block_comment = true;
            }
            continue;
        }

        // Single-line comments: // or #
        if line.starts_with("//") || line.starts_with("#") {
            continue;
        }

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
                let mut prompt = parts[1].trim_end_matches(')').trim();
                
                // Remove surrounding quotes if present (so they don't appear in terminal)
                if (prompt.starts_with('"') && prompt.ends_with('"')) || 
                   (prompt.starts_with('\'') && prompt.ends_with('\'')) {
                    prompt = &prompt[1..prompt.len()-1];
                }
                
                // Escape any internal quotes in prompt
                let escaped_prompt = prompt.replace("\"", "\\\"");
                output.push_str(&format!(
                    "{}print!(\"{}\"); io::stdout().flush().unwrap(); let mut {} = String::new(); io::stdin().read_line(&mut {}).unwrap(); {} = {}.trim().to_string();\n",
                    indent, escaped_prompt, var_name, var_name, var_name, var_name
                ));
            } else {
                // Replace rand() with simple_rand() in variable declarations
                let stmt_with_rand = stmt.replace("rand(", "simple_rand(");
                output.push_str(&format!("{}let mut {};\n", indent, stmt_with_rand));
            }
        }
        // if, else, while, closing braces
        else if line.starts_with("if ") && line.ends_with("{") {
            let condition = &line[3..line.len()-1].trim();
            output.push_str(&format!("{}if {} {{\n", indent, condition));
            indent_level += 1;
        } else if line == "} else {" {
            indent_level -= 1;
            let indent = "    ".repeat(indent_level);
            output.push_str(&format!("{}}} else {{\n", indent));
            indent_level += 1;
        } else if line.starts_with("else") && line.ends_with("{") {
            // This shouldn't happen if } else { is on same line, but keep for safety
            indent_level -= 1;
            let indent = "    ".repeat(indent_level);
            output.push_str(&format!("{}else {{\n", indent));
            indent_level += 1;
        } else if line.starts_with("while ") && line.ends_with("{") {
            let condition = &line[6..line.len()-1].trim();
            output.push_str(&format!("{}while {} {{\n", indent, condition));
            indent_level += 1;
        } else if line == "}" {
            indent_level -= 1;
            output.push_str(&format!("{}}}\n", "    ".repeat(indent_level)));
        }
        // assignment without var (must come before general statement handling)
        else if line.ends_with("--") && line.contains("=") && !line.starts_with("var ") {
            let stmt = &line[..line.len() - 2];
            // Replace rand() with simple_rand() in assignments
            let stmt_with_rand = stmt.replace("rand(", "simple_rand(");
            output.push_str(&format!("{}{};\n", indent, stmt_with_rand));
        }
        // other statements ending with --
        else if line.ends_with("--") {
            let stmt = &line[..line.len() - 2];
            // Replace rand() with simple_rand() in other statements
            let stmt_with_rand = stmt.replace("rand(", "simple_rand(");
            output.push_str(&format!("{}{};\n", indent, stmt_with_rand));
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
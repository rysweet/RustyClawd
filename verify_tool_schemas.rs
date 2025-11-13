#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! serde_json = "1.0"
//! rustyclawd-cli = { path = "crates/cli" }
//! ```

use rustyclawd_cli::tool_definitions::get_all_tool_definitions;

fn main() {
    println!("=== Verifying Tool Schemas ===\n");

    let tools = get_all_tool_definitions();

    println!("Found {} tool definitions\n", tools.len());

    let mut all_ok = true;

    for tool in tools {
        println!("Tool: {}", tool.name);

        // Check for required field
        match tool.input_schema.get("required") {
            None => {
                println!("  ❌ MISSING 'required' field!");
                all_ok = false;
            }
            Some(required) => {
                match required.as_array() {
                    None => {
                        println!("  ❌ 'required' is not an array!");
                        all_ok = false;
                    }
                    Some(arr) => {
                        println!("  ✓ Required fields: {:?}", arr);
                    }
                }
            }
        }

        // Print full schema
        if let Ok(schema_str) = serde_json::to_string_pretty(&tool.input_schema) {
            println!("  Schema: {}", schema_str);
        }
        println!();
    }

    if all_ok {
        println!("\n✓✓✓ ALL TOOLS HAVE PROPER 'required' FIELDS! ✓✓✓");
        std::process::exit(0);
    } else {
        println!("\n❌❌❌ SOME TOOLS ARE MISSING 'required' FIELDS! ❌❌❌");
        std::process::exit(1);
    }
}

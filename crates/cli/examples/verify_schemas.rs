//! Standalone example to verify tool schemas have required fields

// Examples need to reference the crate differently
use rustyclawd::tool_definitions::get_all_tool_definitions;
use rustyclawd_core::client::ToolDefinition;

fn main() {
    println!("=== Verifying Tool Schemas ===\n");

    let tools = get_all_tool_definitions();

    println!("Found {} tool definitions\n", tools.len());

    let mut all_ok = true;

    for tool in &tools {
        println!("Tool: {}", tool.name);

        // Check for required field
        match tool.input_schema.get("required") {
            None => {
                println!("  ❌ MISSING 'required' field!");
                all_ok = false;
            }
            Some(required) => match required.as_array() {
                None => {
                    println!("  ❌ 'required' is not an array!");
                    all_ok = false;
                }
                Some(arr) => {
                    println!("  ✓ Required fields: {:?}", arr);
                }
            },
        }
        println!();
    }

    // Also check serialization round-trip
    println!("=== Checking Serialization Round-Trip ===\n");

    for tool in &tools {
        let serialized = serde_json::to_string(&tool).expect("Should serialize");
        let deserialized: ToolDefinition =
            serde_json::from_str(&serialized).expect("Should deserialize");

        if deserialized.input_schema.get("required").is_none() {
            println!(
                "❌ Tool '{}' lost 'required' field during serialization!",
                tool.name
            );
            all_ok = false;
        } else {
            println!("✓ Tool '{}' serialization OK", tool.name);
        }
    }

    println!();
    if all_ok {
        println!("✓✓✓ ALL TOOLS HAVE PROPER 'required' FIELDS! ✓✓✓");
        println!("✓✓✓ SERIALIZATION PRESERVES 'required' FIELDS! ✓✓✓");
        std::process::exit(0);
    } else {
        println!("❌❌❌ SOME TOOLS ARE MISSING 'required' FIELDS! ❌❌❌");
        std::process::exit(1);
    }
}

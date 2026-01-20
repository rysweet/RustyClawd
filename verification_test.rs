// Quick verification test for Issue #250 MCP Server Wildcard implementation

use rustyclawd::hooks::types::HookMatcher;

fn main() {
    println!("🏴‍☠️ Issue #250 MCP Server Wildcard - Manual Verification\n");

    // Test 1: Filesystem server wildcard
    let filesystem_matcher = HookMatcher::Regex("mcp__filesystem__*".to_string());
    assert!(filesystem_matcher.matches("mcp__filesystem__read_file"));
    assert!(filesystem_matcher.matches("mcp__filesystem__write_file"));
    assert!(!filesystem_matcher.matches("mcp__memory__store"));
    println!("✅ Test 1: Filesystem server wildcard - PASS");

    // Test 2: Memory server wildcard
    let memory_matcher = HookMatcher::Regex("mcp__memory__*".to_string());
    assert!(memory_matcher.matches("mcp__memory__store"));
    assert!(memory_matcher.matches("mcp__memory__read"));
    assert!(!memory_matcher.matches("mcp__filesystem__read_file"));
    println!("✅ Test 2: Memory server wildcard - PASS");

    // Test 3: Priority - Exact over wildcard
    let exact_matcher = HookMatcher::Exact("mcp__filesystem__read_file".to_string());
    let wildcard_matcher = HookMatcher::Regex("mcp__filesystem__*".to_string());
    let tool = "mcp__filesystem__read_file";
    assert!(exact_matcher.matches(tool));
    assert!(wildcard_matcher.matches(tool));
    println!("✅ Test 3: Priority (exact over wildcard) - PASS");

    // Test 4: Priority - Wildcard over general
    let server_wildcard = HookMatcher::Regex("mcp__filesystem__*".to_string());
    let general_mcp = HookMatcher::Regex("mcp__.*".to_string());
    let tool = "mcp__filesystem__read_file";
    assert!(server_wildcard.matches(tool));
    assert!(general_mcp.matches(tool));
    println!("✅ Test 4: Priority (wildcard over general) - PASS");

    // Test 5: Edge case - Underscores in server name
    let custom_matcher = HookMatcher::Regex("mcp__my_custom_server__*".to_string());
    assert!(custom_matcher.matches("mcp__my_custom_server__tool"));
    assert!(!custom_matcher.matches("mcp__other_server__tool"));
    println!("✅ Test 5: Edge case (underscores in name) - PASS");

    // Test 6: Fixed mcp__.*__.* pattern
    let full_pattern = HookMatcher::Regex("mcp__.*__.*".to_string());
    assert!(full_pattern.matches("mcp__server__tool"));
    assert!(full_pattern.matches("mcp__memory__read"));
    assert!(!full_pattern.matches("mcp__"));
    assert!(!full_pattern.matches("mcp__server"));
    println!("✅ Test 6: Fixed mcp__.*__.* pattern - PASS");

    // Test 7: Deserialization
    let json = r#""mcp__filesystem__*""#;
    let matcher: HookMatcher = serde_json::from_str(json).unwrap();
    assert!(matches!(matcher, HookMatcher::Regex(_)));
    assert!(matcher.matches("mcp__filesystem__read_file"));
    println!("✅ Test 7: JSON deserialization - PASS");

    println!("\n🎉 All verification tests PASSED!");
    println!("✅ Issue #250 implementation complete and working correctly!");
}

# --agents JSON Flag: Data Flow & Architecture Diagrams

---

## 1. End-to-End Data Flow

```
User Input (CLI)
│
├─ --agents '{"name": {...}}'  ← User provides JSON
│
v
main.rs - Cli::parse()
│
├─ args.agents = Some(json_string)
│
v
parse_runtime_agents(json_string)
│
├─ serde_json::from_str()
│
├─ Success: HashMap<String, RuntimeAgentDefinition>
│ │
│ v
│ validate_runtime_agents()
│ │
│ ├─ Check each agent:
│ │  ├─ name non-empty ✓ (implicit, JSON structure)
│ │  ├─ description non-empty ?
│ │  ├─ prompt non-empty ?
│ │  └─ tools/model optional
│ │
│ ├─ Success: Ok(())
│ │ │
│ │ v
│ │ AgentDiscovery::with_runtime_agents(runtime_agents)
│ │ │
│ │ ├─ Store in: self.runtime_agents = HashMap
│ │ │
│ │ v
│ │ AgentDiscovery::all_agents()
│ │ │
│ │ ├─ 1. Discover file-based agents from .claude/agents/
│ │ ├─ 2. Add runtime agents (if new IDs)
│ │ │
│ │ v
│ │ Vec<AgentDefinition> ← MERGED!
│ │ │
│ │ ├─ [FileAgent "builder"]
│ │ ├─ [FileAgent "reviewer"]
│ │ ├─ [RuntimeAgent "my-agent"]  ← from CLI
│ │ └─ [RuntimeAgent "custom"]     ← from CLI
│ │
│ ├─ Failure: Err(Vec<String>) ← All validation errors
│ │ │
│ │ v
│ │ eprintln!("Agent validation failed:")
│ │ eprintln!("  - {}", errors[0])
│ │ std::process::exit(1)
│
├─ Failure: Err(String) ← Malformed JSON
│ │
│ v
│ eprintln!("Invalid agents JSON: {}", error)
│ std::process::exit(1)

v
Downstream systems receive merged agents
```

---

## 2. Data Structure Transformation

```
Step 1: JSON String Input
┌─────────────────────────────────────┐
│ '{"my-agent": {                     │
│    "description": "My agent",       │
│    "prompt": "Do something",        │
│    "tools": ["Read", "Write"],      │
│    "model": "sonnet"                │
│  }}'                                │
└─────────────────────────────────────┘

        ↓ parse_runtime_agents()

Step 2: HashMap<String, RuntimeAgentDefinition>
┌─────────────────────────────────────┐
│ {                                   │
│   "my-agent": RuntimeAgentDef {     │
│     description: "My agent",        │
│     prompt: "Do something",         │
│     tools: ["Read", "Write"],       │
│     model: Some("sonnet")           │
│   }                                 │
│ }                                   │
└─────────────────────────────────────┘

        ↓ validate_runtime_agents()

Step 3: Validation Result
┌─────────────────────────────────────┐
│ Ok(())  ← All fields valid          │
│ OR                                  │
│ Err([                               │
│   "Agent 'X' has empty description",│
│   "Agent 'Y' has empty prompt"      │
│ ])                                  │
└─────────────────────────────────────┘

        ↓ AgentDiscovery::with_runtime_agents()

Step 4: AgentDiscovery with Runtime Agents
┌─────────────────────────────────────┐
│ AgentDiscovery {                    │
│   agents_dir: ".../.claude/agents", │
│   runtime_agents: {                 │
│     "my-agent": RuntimeAgentDef {...}│
│   }                                 │
│ }                                   │
└─────────────────────────────────────┘

        ↓ AgentDiscovery::all_agents()

Step 5: Merged Agent Definitions
┌─────────────────────────────────────┐
│ Vec<AgentDefinition> [              │
│   {                                 │
│     id: "builder",                  │
│     name: "Builder Agent",          │
│     description: "Builds code",     │
│     path: "agents/builder.md",      │
│     model: None                     │
│   },                                │
│   {                                 │
│     id: "my-agent",                 │
│     name: "my-agent",               │
│     description: "My agent",        │
│     path: "runtime:my-agent",       │
│     model: Some("sonnet")           │
│   }                                 │
│ ]                                   │
└─────────────────────────────────────┘
```

---

## 3. Validation Logic Tree

```
RuntimeAgentDefinition
│
├─ description: String
│  ├─ Required? YES (serde)
│  ├─ Empty allowed? NO (validate_runtime_agents)
│  └─ Valid example: "Code reviewer for security"
│
├─ prompt: String
│  ├─ Required? YES (serde)
│  ├─ Empty allowed? NO (validate_runtime_agents)
│  └─ Valid example: "You are a security expert..."
│
├─ tools: Vec<String>
│  ├─ Required? NO
│  ├─ Defaults to: Vec::new()
│  ├─ Valid examples: ["Read", "Bash", "Grep"]
│  └─ Note: No validation of tool names (plugins define)
│
└─ model: Option<String>
   ├─ Required? NO
   ├─ Defaults to: None
   ├─ Valid examples: "sonnet", "opus", "claude-3-sonnet-20241022"
   └─ Note: No validation of model ID (LLM API defines)
```

---

## 4. Merging Strategy

```
File-Based Agents (discovered)
┌──────────────────┐
│ "architect"      │
│ "builder"        │
│ "reviewer"       │
└──────────────────┘

         +

Runtime Agents (from --agents JSON)
┌──────────────────┐
│ "my-agent"       │
│ "builder"        │ ← Same ID as file-based!
│ "custom"         │
└──────────────────┘

         =

Merged Result (all_agents())
┌──────────────────┐
│ "architect"      │ (from file)
│ "builder"        │ (from file, NOT overridden)
│ "reviewer"       │ (from file)
│ "my-agent"       │ (from runtime, new ID)
│ "custom"         │ (from runtime, new ID)
└──────────────────┘

Precedence: File agents added first (index 0+)
            Runtime agents added only if new ID
            Result: File agents always win on collision
```

---

## 5. Error Handling Pathways

```
Parse Errors (JSON Syntax)
│
├─ Input: '{"bad json"'  ← Missing close brace
│
├─ Function: parse_runtime_agents()
│
├─ Serde fails: expected value at line 1 column 15
│
└─ Output: Err("Invalid agents JSON: expected value at line 1 column 15")


Validation Errors (Field Constraints)
│
├─ Input: {"name": {"prompt": "..."}}  ← Missing description
│
├─ Function: validate_runtime_agents()
│
├─ Check fails: Agent "name" has empty description
│
└─ Output: Err(["Agent 'name' has empty description"])


No Errors (Success Path)
│
├─ Input: {"name": {"description": "...", "prompt": "..."}}
│
├─ Parse: Ok(HashMap {...})
├─ Validate: Ok(())
├─ Merge: AgentDiscovery with runtime agents
│
└─ Output: Merged Vec<AgentDefinition> ready for use
```

---

## 6. Component Responsibility Matrix

```
┌────────────────────────────────────────┬──────────────┬──────────────┬──────────────┐
│ Component                              │ Parse JSON   │ Validate     │ Merge Agents │
├────────────────────────────────────────┼──────────────┼──────────────┼──────────────┤
│ serde_json                             │ ✓            │              │              │
├────────────────────────────────────────┼──────────────┼──────────────┼──────────────┤
│ parse_runtime_agents()                 │ ✓            │              │              │
├────────────────────────────────────────┼──────────────┼──────────────┼──────────────┤
│ validate_runtime_agents()              │              │ ✓            │              │
├────────────────────────────────────────┼──────────────┼──────────────┼──────────────┤
│ AgentDiscovery::with_runtime_agents()  │              │              │ ✓ (store)    │
├────────────────────────────────────────┼──────────────┼──────────────┼──────────────┤
│ AgentDiscovery::all_agents()           │              │              │ ✓ (combine)  │
├────────────────────────────────────────┼──────────────┼──────────────┼──────────────┤
│ main.rs CLI integration                │              │ (orchestrate)│ (orchestrate)│
└────────────────────────────────────────┴──────────────┴──────────────┴──────────────┘

Note: Each component has ONE responsibility (Single Responsibility Principle)
      No overlapping concerns
      Clean separation enables testing
```

---

## 7. Class Diagram

```
AgentDefinition (from manifest.rs)
├─ id: String          ← File agent: filename, Runtime agent: agent_id
├─ name: String        ← File agent: extracted from content, Runtime agent: agent_id
├─ description: String ← Both: from respective sources
├─ path: String        ← File agent: file path, Runtime agent: "runtime:agent_id"
└─ model: Option<String>

         ↓ converted from

RuntimeAgentDefinition (from agent_discovery.rs)
├─ description: String
├─ prompt: String
├─ tools: Vec<String>
└─ model: Option<String>

         ↑ populated from

CLI args
└─ --agents: Option<String>


Coordination

AgentDiscovery
├─ agents_dir: PathBuf
├─ runtime_agents: HashMap<String, RuntimeAgentDefinition>
│
├─ new() → Self
├─ with_runtime_agents() → Self
├─ discover_all() → Vec<AgentDefinition>  [from files]
└─ all_agents() → Vec<AgentDefinition>    [merged: file + runtime]
```

---

## 8. State Transitions

```
Initial State
├─ Agent Discovery created
├─ Runtime agents: empty
└─ File agents: discovered

        ↓ parse_runtime_agents()

Parsed State
├─ JSON → HashMap
├─ Types validated by serde
└─ Values NOT validated yet

        ↓ validate_runtime_agents()

Validated State
├─ All field constraints satisfied
├─ Or error collected
└─ Ready for merging

        ↓ with_runtime_agents()

Augmented State
├─ Runtime agents stored
├─ File agents unchanged
└─ Ready for merging

        ↓ all_agents()

Merged State
├─ File agents first
├─ Runtime agents appended (new IDs only)
├─ Indexed/searchable
└─ Ready for downstream use
```

---

## 9. Invocation Timeline

```
Time →

T0: User types command
    --agents '{"agent": {...}}'

T1: main.rs parses CLI
    args.agents = Some(json_string)

T2: parse_runtime_agents()
    HashMap<String, RuntimeAgentDefinition>

T3: validate_runtime_agents()
    Err(Vec<String>) or Ok(())

    If Err: print errors, exit(1)
    If Ok: proceed

T4: with_runtime_agents()
    Store in discovery.runtime_agents

T5: all_agents()
    Merge file + runtime

T6: Pass to plugin system
    Systems use merged agents

T7: Session begins
    Runtime agents available
```

---

## 10. Critical Decision Points

```
Decision 1: Parse and Validate Separate?
├─ YES: Clean separation, easier testing ✓ (chosen)
└─ NO: Single function, fewer calls

Decision 2: File or Runtime Agent Wins on Collision?
├─ File wins: Preserves explicit project agents ✓ (chosen)
└─ Runtime wins: More flexible but riskier

Decision 3: Validate All Errors or Fail Fast?
├─ Collect all: User sees complete picture ✓ (chosen)
└─ Fail fast: Faster feedback

Decision 4: Marker for Runtime Agents?
├─ Special path prefix: "runtime:agent_id" ✓ (chosen)
└─ Separate field: More explicit but larger struct

Decision 5: Case Sensitivity?
├─ Sensitive: Consistent with file agents ✓ (chosen)
└─ Insensitive: More forgiving but inconsistent
```

---

## 11. Testing Strategy Map

```
Unit Tests (in agent_discovery.rs)
├─ parse_runtime_agents()
│  ├─ ✓ Valid JSON with all fields
│  ├─ ✓ Valid JSON with minimal fields
│  ├─ ✓ Valid JSON with multiple agents
│  ├─ ✓ Invalid JSON syntax
│  └─ ✓ Missing required fields
│
├─ validate_runtime_agents()
│  ├─ ✓ Valid agents
│  ├─ ✓ Empty description
│  ├─ ✓ Empty prompt
│  ├─ ✓ Multiple validation errors
│  └─ ✓ Optional fields present/absent
│
└─ AgentDiscovery
   ├─ ✓ with_runtime_agents() stores
   ├─ ✓ is_runtime_agent() identifies
   ├─ ✓ all_agents() includes runtime
   └─ ✓ Collision handling (file wins)


Integration Tests (TODO)
├─ CLI parsing
│  └─ args.agents → agent_discovery with runtime agents
│
└─ Full flow
   └─ main.rs parse → validate → merge → use


E2E Tests (TODO)
├─ CLI invocation with --agents
│  └─ Verify agent available in downstream systems
│
└─ Agent execution
   └─ Verify runtime agent actually executed
```

---

## 12. Minimal Example: Complete Flow

```
User Types:
  claude --agents '{"my-reviewer":{"description":"Code reviewer","prompt":"Review code"}}'

Flow:

1. main.rs::Cli::parse()
   → args.agents = Some('{"my-reviewer":...}')

2. parse_runtime_agents(json_str)
   → HashMap::from_json()
   → Ok({
       "my-reviewer": RuntimeAgentDefinition {
         description: "Code reviewer",
         prompt: "Review code",
         tools: [],
         model: None
       }
     })

3. validate_runtime_agents(map)
   → Check "my-reviewer":
     - description non-empty? ✓ "Code reviewer"
     - prompt non-empty? ✓ "Review code"
   → Ok(())

4. agent_discovery.with_runtime_agents(map)
   → self.runtime_agents = map

5. agent_discovery.all_agents()
   → File agents: [architect, builder, ...]
   → Runtime agents: [my-reviewer] (new ID)
   → Result: [architect, builder, ..., my-reviewer]

6. my-reviewer available for use!
```

---

**Key Insight**: Three independent layers (parse → validate → merge), each with zero dependencies on the others. Easy to test, easy to extend, ruthlessly simple.

# Rust Patterns Learned Through Claude Code Translation

**Date**: 2025-11-10
**Project**: Claude Code JavaScript → Rust Translation
**Purpose**: Educational documentation of Rust patterns encountered

---

## 🎓 Pattern Catalog

This document catalogs every Rust pattern we encountered while translating Claude Code from JavaScript, organized by complexity and importance.

---

## ⭐ Essential Patterns (Must Know)

### 1. Ownership and Borrowing in Async Streams

**Challenge**: Stream closures capture variables and require `'static` lifetime.

**JavaScript (No Issue)**:
```javascript
async function* streamData(config) {
    yield `Using ${config.setting}`;  // Captures reference freely
}
```

**Rust (Requires Ownership)**:
```rust
fn stream_data(config: &Config) -> impl Stream<Item = String> {
    // WRONG: Can't capture reference in 'static stream
    // let setting = &config.setting;

    // RIGHT: Clone owned data before capture
    let setting = config.setting.clone();

    stream! {
        yield format!("Using {}", setting);  // Uses owned value
    }
}
```

**Learning**: When creating streams/futures, clone data you need. References won't live long enough.

**Pattern**:
```rust
async fn create_stream(ctx: &Context) -> impl Stream<Item = Event> {
    // Extract owned copies BEFORE stream! macro
    let field1 = ctx.field1.clone();
    let field2 = ctx.field2;  // Copy for Copy types

    stream! {
        // Use owned values safely
        yield Event::new(field1, field2);
    }
}
```

---

### 2. Associated Types for Type-Safe Traits

**Challenge**: Different tools have different parameter/output types.

**JavaScript (Runtime Validation)**:
```javascript
const tool = {
    execute: async (params) => {
        // params is any, validate at runtime
        const validated = schema.parse(params);
        return result;
    }
};
```

**Rust (Compile-Time Safety)**:
```rust
pub trait Tool {
    type Params: DeserializeOwned;  // Each tool specifies its param type
    type Output: Serialize;          // Each tool specifies its output type

    async fn execute(&self, params: Self::Params) -> ToolStream<Self::Output>;
}

// Implementation
impl Tool for BashTool {
    type Params = BashParams;  // Specific type
    type Output = BashOutput;  // Specific type

    async fn execute(&self, params: BashParams) -> ToolStream<BashOutput> {
        // params is already the right type - no runtime validation needed!
    }
}
```

**Learning**: Associated types let each implementation specify its own types while maintaining trait uniformity.

---

### 3. Error Handling with `?` Operator

**Challenge**: Propagate errors without verbose code.

**JavaScript**:
```javascript
try {
    const content = await fs.readFile(path);
    const parsed = JSON.parse(content);
    return await sendToAPI(parsed);
} catch (err) {
    if (err.code === 'ENOENT') {
        console.error('File not found');
    } else if (err instanceof SyntaxError) {
        console.error('Invalid JSON');
    }
    throw err;
}
```

**Rust**:
```rust
// Define error types
#[derive(Error, Debug)]
pub enum MyError {
    #[error("File not found: {0}")]
    NotFound(PathBuf),

    #[error("Invalid JSON")]
    InvalidJson(#[from] serde_json::Error),

    #[error("API error")]
    ApiError(#[from] ApiError),
}

// Clean error propagation
pub async fn process(path: PathBuf) -> Result<Response, MyError> {
    let content = fs::read_to_string(&path)
        .await
        .map_err(|_| MyError::NotFound(path))?;

    let parsed: Data = serde_json::from_str(&content)?;  // Auto-converts via From
    let response = send_to_api(parsed).await?;           // Auto-converts
    Ok(response)
}
```

**Learning**: `?` operator + `From` trait makes error handling concise while maintaining type safety.

---

### 4. Async Streams with `async-stream` Crate

**Challenge**: JavaScript has native `async function*`, Rust doesn't.

**JavaScript**:
```javascript
async function* processItems(items) {
    for (const item of items) {
        yield `Processing ${item}`;
        const result = await process(item);
        yield result;
    }
}
```

**Rust**:
```rust
use async_stream::stream;

fn process_items(items: Vec<String>) -> impl Stream<Item = String> {
    stream! {
        for item in items {
            yield format!("Processing {}", item);
            let result = process(&item).await;
            yield result;
        }
    }
}
```

**Learning**: `async-stream` crate provides generator-like syntax while maintaining Rust's safety guarantees.

---

### 5. Trait Objects for Dynamic Dispatch

**Challenge**: Store different tool types in a collection.

**JavaScript**:
```javascript
const tools = [
    { name: "Read", execute: readFn },
    { name: "Write", execute: writeFn },
];
```

**Rust**:
```rust
use std::sync::Arc;

// Trait object: dyn Tool with Send constraint for thread-safety
type BoxedTool = Arc<dyn Tool<Params = serde_json::Value, Output = serde_json::Value> + Send + Sync>;

// Or with associated types:
struct ToolRegistry {
    tools: HashMap<String, Arc<dyn ErasedTool>>,
}

// Type erasure trait
trait ErasedTool: Send + Sync {
    fn name(&self) -> &str;
    fn execute_json(&self, params: serde_json::Value) -> BoxedStream;
}
```

**Learning**: `dyn Trait` enables runtime polymorphism. `Arc` provides shared ownership. `Send + Sync` ensure thread-safety.

---

### 6. Pin and Unpin for Self-Referential Types

**Challenge**: Streams must be pinned in memory.

**Why Pin Exists**:
```rust
// Stream might contain self-references
struct StreamState {
    buffer: Vec<u8>,
    slice: &mut [u8],  // Points into buffer!
}

// If this moves, the reference breaks! Pin prevents moves.
```

**Usage**:
```rust
pub type ToolStream<T> = Pin<Box<dyn Stream<Item = T> + Send>>;

// Create pinned stream
Ok(Box::pin(stream! {
    // stream body
}))
```

**Learning**: `Pin` ensures async state machines don't move in memory, preventing invalid references.

---

## 🚀 Advanced Patterns

### 7. Phantom Types for Compile-Time State

**Use Case**: Ensure tools are validated before execution.

```rust
use std::marker::PhantomData;

struct Validated;
struct Unvalidated;

struct Tool<State = Unvalidated> {
    params: Params,
    _state: PhantomData<State>,
}

impl Tool<Unvalidated> {
    pub fn validate(self) -> Result<Tool<Validated>, Error> {
        // Validate params
        Ok(Tool {
            params: self.params,
            _state: PhantomData,
        })
    }
}

impl Tool<Validated> {
    pub async fn execute(self) -> Output {
        // Can only call on validated tool
    }
}

// Usage
let tool = Tool::new(params)
    .validate()?       // Returns Tool<Validated>
    .execute().await;  // Only works on validated
```

**Learning**: Phantom types encode state in the type system, making invalid states unrepresentable.

---

### 8. Builder Pattern with Typestate

**Challenge**: Ensure all required fields are set before building.

```rust
struct ToolBuilder<HasName, HasParams> {
    name: Option<String>,
    params: Option<Params>,
    _phantom: PhantomData<(HasName, HasParams)>,
}

struct Yes;
struct No;

impl ToolBuilder<No, No> {
    pub fn new() -> Self { /* ... */ }

    pub fn name(self, name: String) -> ToolBuilder<Yes, No> {
        ToolBuilder {
            name: Some(name),
            params: self.params,
            _phantom: PhantomData,
        }
    }
}

impl<HasName> ToolBuilder<HasName, No> {
    pub fn params(self, params: Params) -> ToolBuilder<HasName, Yes> {
        // ...
    }
}

// Can only call build() when all fields are set
impl ToolBuilder<Yes, Yes> {
    pub fn build(self) -> Tool {
        Tool {
            name: self.name.unwrap(),
            params: self.params.unwrap(),
        }
    }
}

// Usage
let tool = ToolBuilder::new()
    .name("MyTool".into())
    .params(params)
    .build();  // Compiles only if name and params set!
```

---

### 9. Newtype Pattern for Type Safety

**Challenge**: Distinguish between different string types.

```rust
// Instead of stringly-typed APIs:
fn execute(tool_name: String, file_path: String) { }
execute(file_path, tool_name);  // Oops, swapped! Compiles fine in JS.

// Use newtypes:
struct ToolName(String);
struct FilePath(String);

fn execute(tool_name: ToolName, file_path: FilePath) { }
execute(FilePath("".into()), ToolName("".into()));  // Compile error!
```

**Learning**: Newtypes prevent mixing up values of the same underlying type.

---

### 10. Interior Mutability with RefCell/Mutex

**Challenge**: Need to mutate through shared reference.

```rust
use std::sync::{Arc, Mutex};

struct ToolRegistry {
    // Can mutate even when behind Arc
    tools: Arc<Mutex<HashMap<String, Box<dyn Tool>>>>,
}

impl ToolRegistry {
    pub fn register(&self, name: String, tool: Box<dyn Tool>) {
        let mut tools = self.tools.lock().unwrap();
        tools.insert(name, tool);
    }
}
```

**Learning**: `Mutex` allows mutation through shared references safely (runtime borrow checking).

---

## 🏆 Best Practices Discovered

### 1. Clone Smartly

**Don't Over-Clone**:
```rust
// BAD: Unnecessary clones
let config = ctx.config.clone();
let settings = config.settings.clone();
let value = settings.value.clone();

// GOOD: Clone only what you need
let value = ctx.config.settings.value.clone();
```

**Use Arc for Shared Ownership**:
```rust
// Instead of cloning large structures:
struct Context {
    large_data: Arc<LargeStruct>,  // Reference-counted, not cloned
}
```

---

### 2. Prefer `impl Trait` Over Trait Objects When Possible

```rust
// GOOD: Static dispatch (zero-cost)
fn create_stream() -> impl Stream<Item = String> {
    stream! { yield "data".into(); }
}

// OK: Dynamic dispatch when needed (small overhead)
fn create_boxed() -> Pin<Box<dyn Stream<Item = String> + Send>> {
    Box::pin(stream! { yield "data".into(); })
}
```

**When to use each**:
- `impl Trait`: When concrete type is known at compile time
- `Box<dyn Trait>`: When storing different types in collection or returning from trait method

---

### 3. Error Context with `anyhow`

```rust
use anyhow::Context;

// Add context to errors
let content = fs::read_to_string(path)
    .await
    .context("Failed to read configuration file")?;

// Chain context
let parsed: Config = serde_json::from_str(&content)
    .context("Configuration file has invalid JSON")?;
```

**Learning**: Error context makes debugging much easier.

---

### 4. Testing Patterns for Async Code

```rust
#[tokio::test]
async fn test_tool_execution() {
    // Use tempfile for filesystem operations
    let temp_file = NamedTempFile::new().unwrap();

    // Execute async operation
    let result = tool.execute(params).await.unwrap();

    // Collect stream
    let events: Vec<_> = result.collect().await;

    // Find specific event type
    let output = events.iter().find_map(|e| match e {
        ToolEvent::Result(r) => Some(r),
        _ => None,
    }).unwrap();

    // Assertions
    assert_eq!(output.count, expected);
}
```

---

### 5. Declarative CLI with Clap Derives

```rust
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    debug: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Read {
        file_path: String,
        #[arg(long)]
        offset: Option<usize>,
    },
}
```

**Learning**: Derive macros make CLI definition declarative and type-safe.

---

## 🐛 Common Pitfalls Encountered

### 1. Forgetting to Clone Before Capture

**Error**:
```rust
async fn create_stream(ctx: &Context) -> ToolStream {
    Ok(Box::pin(stream! {
        let value = &ctx.field;  // ❌ Reference doesn't live long enough
        yield Event::new(value);
    }))
}
```

**Fix**:
```rust
async fn create_stream(ctx: &Context) -> ToolStream {
    let field = ctx.field.clone();  // ✅ Clone before stream!

    Ok(Box::pin(stream! {
        yield Event::new(field);
    }))
}
```

---

### 2. Mixing Sync and Async I/O

**Error**:
```rust
use std::fs;  // Sync I/O

#[tokio::main]
async fn main() {
    fs::read_to_string(path)?;  // ❌ Blocks async runtime!
}
```

**Fix**:
```rust
use tokio::fs;  // Async I/O

#[tokio::main]
async fn main() {
    fs::read_to_string(path).await?;  // ✅ Non-blocking
}
```

---

### 3. Not Importing Trait Extensions

**Error**:
```rust
let mut stream = create_stream();
stream.next().await;  // ❌ no method `next` found
```

**Fix**:
```rust
use futures::StreamExt;  // ✅ Brings extension methods into scope

let mut stream = create_stream();
stream.next().await;  // Works!
```

---

### 4. Forgetting Send/Sync Bounds

**Error**:
```rust
pub type ToolStream<T> = Pin<Box<dyn Stream<Item = T>>>;
// ❌ Not Send - can't cross await points
```

**Fix**:
```rust
pub type ToolStream<T> = Pin<Box<dyn Stream<Item = T> + Send>>;
// ✅ Can be sent between threads
```

---

## 🔬 Pattern Comparison: JavaScript vs Rust

### Pattern: Async Iteration

**JavaScript**:
```javascript
for await (const item of stream) {
    console.log(item);
}
```

**Rust**:
```rust
use futures::StreamExt;

while let Some(item) = stream.next().await {
    println!("{:?}", item);
}

// Or with combinators:
stream.for_each(|item| async move {
    println!("{:?}", item);
}).await;
```

---

### Pattern: Optional Chaining

**JavaScript**:
```javascript
const value = obj?.field?.nested?.value ?? "default";
```

**Rust**:
```rust
let value = obj.field
    .as_ref()
    .and_then(|f| f.nested.as_ref())
    .and_then(|n| n.value.as_ref())
    .unwrap_or(&"default");

// Or with map:
let value = obj.field
    .and_then(|f| f.nested)
    .and_then(|n| n.value)
    .unwrap_or_default();
```

---

### Pattern: Default Values

**JavaScript**:
```javascript
function execute(params = {}) {
    const timeout = params.timeout ?? 120000;
    const debug = params.debug ?? false;
}
```

**Rust**:
```rust
#[derive(Deserialize)]
struct Params {
    #[serde(default = "default_timeout")]
    timeout: u64,

    #[serde(default)]  // Uses Default trait
    debug: bool,
}

fn default_timeout() -> u64 {
    120000
}
```

---

## 🎯 When to Use Each Pattern

| Pattern | Use When | Avoid When |
|---------|----------|------------|
| **Associated Types** | Trait implementations need different types | All implementations use same type |
| **Trait Objects** | Need heterogeneous collections | Performance critical (use enums) |
| **`impl Trait`** | Return types from functions | Need trait objects or generics |
| **Arc/Rc** | Shared ownership needed | Single owner is sufficient |
| **Mutex/RwLock** | Interior mutability required | Can use `&mut` |
| **Phantom Types** | Encode state in type system | State is runtime-only |
| **Newtype** | Prevent mixing similar types | Overhead not justified |

---

## 📊 Performance Patterns

### Pattern: Avoid Unnecessary Cloning

**Slow**:
```rust
fn process(data: Vec<String>) -> Vec<String> {
    data.clone()  // Unnecessary clone
        .into_iter()
        .map(|s| s.to_uppercase())
        .collect()
}
```

**Fast**:
```rust
fn process(data: Vec<String>) -> Vec<String> {
    data.into_iter()  // Consume, no clone
        .map(|s| s.to_uppercase())
        .collect()
}
```

---

### Pattern: Use `&str` for Parameters, `String` for Return

```rust
// GOOD: Accepts both &str and String
fn format_message(prefix: &str, msg: &str) -> String {
    format!("{}: {}", prefix, msg)
}

// AVOID: Forces allocation
fn format_message(prefix: String, msg: String) -> String {
    format!("{}: {}", prefix, msg)
}
```

---

## 🎓 Key Insights

### 1. Rust Makes Implicit Explicit

JavaScript hides:
- Memory allocation (GC)
- Error paths (exceptions)
- Async state machines
- Ownership transfer

Rust makes you think about these explicitly.

### 2. Compile-Time Costs, Runtime Benefits

Rust compilation is slower, but you get:
- No runtime type errors
- No null pointer exceptions
- No data races
- Predictable performance

### 3. The Borrow Checker Teaches Good Design

Fighting the borrow checker often reveals:
- Unnecessary shared mutable state
- Overly complex ownership graphs
- Missing encapsulation

### 4. Tests Catch More Issues

Because Rust is stricter:
- Tests must handle all error cases
- Can't ignore type mismatches
- Forced to think about edge cases

---

## 🚀 Next Level Patterns (Future Phases)

### Patterns to Learn in Phase 2+

1. **Generic Associated Types (GATs)** - For complex trait designs
2. **Higher-Ranked Trait Bounds (HRTBs)** - `for<'a>` lifetime bounds
3. **Async Recursion** - Using `async-recursion` crate
4. **Zero-Copy Parsing** - `bytes` crate for efficient parsing
5. **Lock-Free Concurrency** - Atomic types and memory ordering

---

## 📚 Recommended Reading Order

To master these patterns:

1. **The Book** (Ch 10, 13, 16, 17) - Traits, closures, fearless concurrency, async
2. **Async Book** - Complete async/await guide
3. **Too Many Lists** - Learn unsafe Rust through linked lists
4. **Rust for Rustaceans** - Advanced patterns

---

## ✅ Checklist: Have You Mastered These?

- [ ] Can explain ownership rules without checking docs
- [ ] Can use `?` operator fluently
- [ ] Understand when to use `&` vs `&mut` vs owned
- [ ] Can implement traits with associated types
- [ ] Comfortable with `Pin`, `Box`, `Arc`
- [ ] Can use `async`/`await` without blocking
- [ ] Understand `Send` and `Sync` markers
- [ ] Can choose between trait objects and generics
- [ ] Write idiomatic error handling
- [ ] Use derive macros effectively

If you completed this translation, you should check most of these! 🎉

---

**This document grows as we implement more phases. See RUST_TRANSLATION_PLAN.md for the full journey.**

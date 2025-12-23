# Rust Patterns and Principles Guide

> **Purpose**: Educational reference for Rust patterns used throughout the Colossus codebase  
> **For**: Learning Rust while working with the code  
> **Updated**: 2025-08-28  

## 📚 Table of Contents

1. [Ownership and Borrowing](#ownership-and-borrowing)
2. [Error Handling](#error-handling)
3. [Async Programming](#async-programming)
4. [Smart Pointers](#smart-pointers)
5. [Traits and Derives](#traits-and-derives)
6. [Type System](#type-system)
7. [Common Patterns](#common-patterns)

---

## Ownership and Borrowing

### String Types {#string-types}

**Pattern**: `String` vs `&str`

```rust
pub struct Config {
    url: String,        // Owned string - this struct owns the data
    name: &'static str, // Borrowed string slice - references data owned elsewhere
}
```

**Why**: 
- `String` is an owned, growable string on the heap
- `&str` is a borrowed reference to a string slice (no allocation)
- Use `String` when you need ownership, `&str` when you just need to read

**In our code**: 
- `base_url: String` in OllamaClient - needs to own the URL throughout its lifetime
- `model: &str` in function params - just needs to read the value temporarily

### Borrowing {#borrowing}

**Pattern**: `&T` (immutable borrow) vs `&mut T` (mutable borrow)

```rust
pub async fn analyze(&self, content: &str) -> Result<String>
//                    ^^^^^ immutable borrow of self
//                            ^^^^ immutable borrow of string
```

**Why**: 
- Borrowing prevents data races at compile time
- Multiple immutable borrows OR one mutable borrow
- References are guaranteed valid by the borrow checker

**In our code**: Most methods take `&self` because they don't modify the struct

---

## Error Handling

### Result Pattern {#error-handling}

**Pattern**: `Result<T, E>` for fallible operations

```rust
pub async fn list_models(&self) -> Result<Vec<OllamaModel>>
//                                  ^^^^^^ Can succeed with Vec or fail with Error
```

**Why**:
- No exceptions in Rust - errors are values
- Forces explicit error handling
- Can't accidentally ignore errors

**In our code**: All I/O operations return `Result`

### Question Mark Operator {#error-context}

**Pattern**: `?` for error propagation with `.context()`

```rust
let response = client
    .send()
    .await
    .context("Failed to send request")?;  // Add context and propagate error
//                                    ^ Returns early if Err, continues if Ok
```

**Why**:
- `?` is shorthand for match { Ok(v) => v, Err(e) => return Err(e.into()) }
- `.context()` from anyhow adds human-readable error messages
- Creates error chain for debugging

**In our code**: Used throughout for clean error handling

---

## Async Programming

### Async/Await {#async-await}

**Pattern**: `async fn` and `.await`

```rust
pub async fn generate_completion(&self, ...) -> Result<String> {
//   ^^^^^ Function returns a Future
    let response = client.send().await;  // Suspend until Future completes
//                              ^^^^^^
}
```

**Why**:
- Non-blocking I/O without callback hell
- Futures are lazy - nothing happens until `.await`
- Can run many async operations concurrently

**In our code**: All network calls are async

### Async Errors {#async-errors}

**Pattern**: Combining `async` with `Result`

```rust
pub async fn health_check(&self) -> Result<HealthStatus>
//   ^^^^^                          ^^^^^^ Two layers: async AND can fail
```

**Why**:
- Async operations often involve I/O that can fail
- Pattern: `async fn() -> Result<T>` is very common
- Caller must both `.await` AND handle errors

---

## Smart Pointers

### Arc Pattern {#arc-pattern}

**Pattern**: `Arc<T>` for shared ownership across threads

```rust
pub struct OllamaClient {
    client: Arc<reqwest::Client>,  // Multiple threads can share this client
}
```

**Why**:
- Arc = Atomic Reference Counting
- Allows multiple owners of same data
- Thread-safe (unlike `Rc<T>`)
- Cloning an Arc is cheap (just increments counter)

**In our code**: HTTP clients are expensive to create, so we share one instance

### Box Pattern (Not in current code, but common)

**Pattern**: `Box<T>` for heap allocation

```rust
Box::new(large_struct)  // Allocate on heap instead of stack
```

**Why**:
- Fixed-size pointer on stack, data on heap
- Useful for recursive types or large data
- Single owner (unlike Arc)

---

## Traits and Derives

### Derive Macros {#derive-macros}

**Pattern**: `#[derive(...)]` for automatic trait implementation

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OllamaModel {
    pub name: String,
}
```

**Why**:
- Compiler generates boilerplate implementations
- `Clone` - enables `.clone()` method
- `Debug` - enables `{:?}` formatting
- `Serialize/Deserialize` - JSON conversion via serde

**In our code**: Most structs derive common traits

### Serde Pattern {#serde}

**Pattern**: Serde traits for serialization

```rust
#[derive(Serialize)]    // Can convert TO JSON
struct Request { ... }

#[derive(Deserialize)]  // Can convert FROM JSON  
struct Response { ... }
```

**Why**:
- Declarative JSON handling
- Works with many formats (JSON, TOML, YAML, etc.)
- Compile-time guarantees

---

## Type System

### Option Type {#option-type}

**Pattern**: `Option<T>` for nullable values

```rust
pub async fn analyze(content: &str, context: Option<&str>) -> Result<String>
//                                            ^^^^^^^^^^^^^ May or may not have context
```

**Why**:
- No null pointer exceptions
- Must explicitly handle None case
- Common methods: `.unwrap()`, `.unwrap_or()`, `.map()`, `.and_then()`

**In our code**: Optional parameters like `system_prompt: Option<&str>`

### Type Aliases (Future improvement)

**Pattern**: Type aliases for clarity

```rust
type DbPool = Arc<PgPool>;
type ApiResult<T> = Result<T, ApiError>;
```

**Why**: Makes complex types more readable

---

## Common Patterns

### Constructor Pattern {#constructors}

**Pattern**: `new()` associated function

```rust
impl OllamaClient {
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }
}
```

**Why**:
- Convention: `new()` creates instances
- Not a special keyword (unlike other languages)
- Can have multiple constructors: `new()`, `from_env()`, `with_config()`

### Impl Blocks {#impl-blocks}

**Pattern**: `impl` for methods and associated functions

```rust
impl OllamaClient {           // Methods for this type
    pub fn new() -> Self {    // Associated function (no self)
    pub fn send(&self) {      // Method (takes self)
    pub fn modify(&mut self) { // Mutable method
}
```

**Why**:
- Separates data (struct) from behavior (impl)
- Can have multiple impl blocks
- Enables extension without modifying original

### Builder Pattern (Future improvement)

**Pattern**: Chainable configuration

```rust
Client::builder()
    .timeout(30)
    .retries(3)
    .build()?
```

**Why**: Ergonomic configuration of complex objects

### Field Init Shorthand {#struct-syntax}

**Pattern**: Shorthand struct initialization

```rust
Self {
    base_url,  // Same as base_url: base_url
}
```

**Why**: Less repetition when variable name matches field name

### Axum Extractors {#extractors}

**Pattern**: Function parameters that extract from HTTP requests

```rust
pub async fn handler(
    State(state): State<AppState>,     // Extract shared state
    Path(id): Path<Uuid>,               // Extract from URL path
    Json(body): Json<RequestType>,     // Parse JSON body
    Query(params): Query<QueryParams>, // Parse query string
) -> Result<Json<Response>, Error>
```

**Why**:
- Type-safe request parsing
- Automatic deserialization
- Composable - can combine multiple extractors
- Fails gracefully with proper HTTP errors

**In our code**: All API handlers use extractors for clean parameter handling

### Rate Limiting Pattern {#rate-limiting}

**Pattern**: Asynchronous delays for external API compliance

```rust
// Rate limiting before external API calls
tokio::time::sleep(self.rate_limit_delay).await;
let response = self.client.get(&url).send().await?;
```

**Why**:
- Respects external API rate limits (arXiv: 1 request per 3 seconds)
- Non-blocking delay using tokio's async sleep
- Prevents service from being blocked by external APIs

**In our code**: ArxivService implements proper rate limiting for arXiv API calls

### XML Deserialization Pattern {#xml-serde}

**Pattern**: Using serde with quick-xml for XML parsing

```rust
#[derive(Debug, Deserialize)]
struct ArxivAtomFeed {
    #[serde(rename = "totalResults")]
    total_results: Option<u32>,
    #[serde(rename = "entry", default)]
    entries: Vec<ArxivAtomEntry>,
}

// Parse XML response
let feed: ArxivAtomFeed = quick_xml::de::from_str(&xml_text)?;
```

**Why**:
- Declarative XML parsing with compile-time guarantees
- `#[serde(rename = "...")]` handles XML naming conventions
- `#[serde(default)]` provides fallbacks for missing fields
- Type-safe conversion from XML to Rust structs

**In our code**: ArxivService parses arXiv's Atom XML responses into typed Rust structures

### Complex Data Transformation Pattern {#data-transformation}

**Pattern**: Converting between different data formats

```rust
fn convert_entries_to_papers(&self, entries: Vec<ArxivAtomEntry>) -> Result<Vec<ArxivPaper>> {
    let mut papers = Vec::new();
    for entry in entries {
        // Extract arXiv ID from URL
        let paper_id = entry.id
            .strip_prefix("http://arxiv.org/abs/")
            .unwrap_or(&entry.id)
            .to_string();
        
        // Convert XML datetime to UTC
        let published = DateTime::parse_from_rfc3339(&entry.published)?
            .with_timezone(&Utc);
        
        papers.push(ArxivPaper { paper_id, published, /* ... */ });
    }
    Ok(papers)
}
```

**Why**:
- Transforms external API format to internal representation
- Handles data validation and type conversions
- Provides clean error handling for malformed data
- Encapsulates complex parsing logic in dedicated functions

**In our code**: ArxivService converts arXiv XML format to our JSON API format

### Integration Test Helper Pattern {#test-helpers}

**Pattern**: Reusable test utilities for DRY testing

```rust
// Test helper for HTTP requests
async fn make_request(app: axum::Router, method: &str, uri: &str, body: Option<String>) -> axum::response::Response {
    let mut request_builder = Request::builder().method(method).uri(uri);
    if body.is_some() {
        request_builder = request_builder.header("content-type", "application/json");
    }
    let request = match body {
        Some(body_content) => request_builder.body(Body::from(body_content)).unwrap(),
        None => request_builder.body(Body::empty()).unwrap(),
    };
    app.oneshot(request).await.unwrap()
}
```

**Why**:
- Reduces code duplication in tests
- Provides consistent test setup across different scenarios
- Makes tests more readable by hiding boilerplate
- Easier to maintain - changes to test setup affect all tests

**In our code**: arxiv_integration_tests.rs uses helper functions for app setup and request handling

---

## Learning Resources

### Where These Patterns Appear in Our Code

1. **OllamaClient** (`/services/ollama_client.rs`):
   - Arc for shared HTTP client
   - async/await for network calls
   - Result for error handling
   - Option for optional parameters

2. **ArxivService** (`/services/arxiv_service.rs`):
   - HTTP Client configuration with timeouts and user agents
   - XML deserialization with serde and quick-xml
   - Rate limiting with tokio::time::sleep
   - Complex data transformation from XML to JSON structs
   - Error handling with anyhow::Context for detailed error chains

3. **AppState** (`/state.rs`):
   - Arc for shared state across handlers
   - Multiple service composition

4. **API Handlers** (`/api/*.rs`):
   - async request handlers
   - Error conversion and propagation
   - JSON serialization with serde

5. **Integration Testing** (`/tests/arxiv_integration_tests.rs`):
   - Test helper functions for DRY testing patterns
   - axum::body::to_bytes for consuming HTTP response bodies
   - Status code checking and response validation
   - Comprehensive test coverage for edge cases and error conditions

### Recommended Learning Order

1. **Start with**: Ownership and borrowing (fundamental to Rust)
2. **Then learn**: Error handling with Result and ?
3. **Next**: Async basics for web services
4. **Finally**: Smart pointers when sharing data

### Common Gotchas for Rust Learners

1. **Fighting the borrow checker**: It's your friend, preventing memory bugs
2. **Overusing `clone()`**: Try borrowing first
3. **Overusing `unwrap()`**: Handle errors properly with `?` or `match`
4. **Confusing `String` and `&str`**: Ownership vs borrowing
5. **Async without `.await`**: Future won't execute

### Quick Debugging Tips

- **"cannot move out of borrowed content"**: You're trying to take ownership of borrowed data
- **"does not live long enough"**: Lifetime issue - data dropped while still referenced
- **"cannot borrow as mutable"**: Already borrowed immutably elsewhere
- **"Future is not Send"**: Can't share across threads - check for non-Send types

---

## Questions to Ask When Reading Code

1. **Who owns this data?** (Look for `String`, `Vec`, `Box`)
2. **Am I borrowing or owning?** (Look for `&`)
3. **Can this fail?** (Look for `Result`)
4. **Is this async?** (Look for `async`/`await`)
5. **Is this shared?** (Look for `Arc`, `Mutex`)

---

**Remember**: The compiler is your teacher. When it complains, it's teaching you about memory safety, concurrency, and correctness. Read the error messages carefully - they're some of the best in any language!
# Rate Limiter

A Rust library providing various rate limiting algorithms for controlling the rate of operations in async applications.

## Features

- **Token Bucket**: Allows bursts up to bucket capacity, with tokens refilled at a steady rate
- **Leaky Bucket**: Smooth rate limiting with a fixed processing rate
- **Fixed Window**: Allows N operations per fixed time window
- **Sliding Window**: Allows N operations per sliding time window
- **Async/Await Support**: Fully compatible with Tokio and other async runtimes
- **Thread Safe**: All rate limiters can be safely shared across threads
- **Easy Integration**: Simple API that wraps any Future

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
rate_limiter = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
futures = "0.3"
```

## Quick Start

### Basic Rate Limiting

```rust
use rate_limiter::{TokenBucket, SlidingWindow, RateLimitedTask};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
   // Create a token bucket with 10 tokens, refilling at 2 tokens per second
   let limiter = TokenBucket::new()
       .bucket_size(10)
       .refill_rate(2.0);

   // Direct usage
   for i in 0..5 {
       limiter.acquire().await;
       println!("Request {} completed", i);
   }

   Ok(())
}
```

### Rate Limiting HTTP Requests

```rust
use rate_limiter::{SlidingWindow, RateLimitedTask};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
   // Allow 100 requests per 60 seconds
   let limiter = SlidingWindow::new()
       .limit(100)
       .window_size(60);

   async fn make_request(id: i32) -> Result<String, String> {
       // Simulate HTTP request
       tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
       Ok(format!("Response {}", id))
   }

   // Rate limit the requests
   let task1 = RateLimitedTask(make_request(1)).limited_by(limiter.clone());
   let task2 = RateLimitedTask(make_request(2)).limited_by(limiter.clone());
   let task3 = RateLimitedTask(make_request(3)).limited_by(limiter.clone());

   let (result1, result2, result3) = futures::join!(task1, task2, task3);
   
   println!("Results: {:?}, {:?}, {:?}", result1, result2, result3);
   Ok(())
}
```

### Concurrent Rate Limiting

```rust
use rate_limiter::{FixedWindow, RateLimitedTask};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
   // Allow 2 requests per 5-second window
   let limiter = FixedWindow::new()
       .limit(2)
       .window_size(5);

   async fn api_call(id: i32) -> i32 {
       println!("Starting API call {}", id);
       tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
       println!("Completed API call {}", id);
       id
   }

   // First two requests go through immediately
   // Third request waits for window reset
   let tasks: Vec<_> = (1..=4)
       .map(|i| RateLimitedTask(api_call(i)).limited_by(limiter.clone()))
       .collect();

   let results = futures::future::join_all(tasks).await;
   println!("All results: {:?}", results);

   Ok(())
}
```

## Rate Limiting Algorithms

### Token Bucket
Best for allowing bursts while maintaining average rate.

```rust
let limiter = TokenBucket::new()
   .bucket_size(10)        // Maximum 10 tokens in bucket
   .refill_rate(2.5);      // Add 2.5 tokens per second
```

**Use cases**: API rate limiting, bandwidth control, burst traffic handling

### Leaky Bucket
Smooths out bursty traffic by processing requests at a constant rate.

```rust
let limiter = LeakyBucket::new()
   .bucket_size(5)         // Queue up to 5 requests
   .leak_rate(1.0);        // Process 1 request per second
```

**Use cases**: Traffic shaping, preventing downstream overload

### Fixed Window
Simple rate limiting with fixed time windows.

```rust
let limiter = FixedWindow::new()
   .limit(100)             // 100 requests
   .window_size(60);       // per 60 seconds
```

**Use cases**: Simple API quotas, basic rate limiting

### Sliding Window
More precise rate limiting with rolling time windows.

```rust
let limiter = SlidingWindow::new()
   .limit(100)             // 100 requests
   .window_size(60);       // per 60 seconds (sliding)
```

**Use cases**: Precise rate limiting, avoiding burst at window boundaries

## API Reference

### TokenBucket
- `new()` - Create a new token bucket
- `bucket_size(size)` - Set maximum tokens in bucket
- `refill_rate(rate)` - Set tokens added per second
- `acquire()` - Wait for and consume one token

### LeakyBucket
- `new()` - Create a new leaky bucket  
- `bucket_size(size)` - Set maximum queue size
- `leak_rate(rate)` - Set processing rate (requests per second)
- `acquire()` - Add request to queue and wait for processing

### FixedWindow
- `new()` - Create a new fixed window limiter
- `limit(count)` - Set maximum requests per window
- `window_size(seconds)` - Set window duration in seconds
- `acquire()` - Wait for available slot in current window

### SlidingWindow
- `new()` - Create a new sliding window limiter
- `limit(count)` - Set maximum requests per window
- `window_size(seconds)` - Set window duration in seconds  
- `acquire()` - Wait for available slot in sliding window

### RateLimitedTask
- `RateLimitedTask(future)` - Wrap a future for rate limiting
- `limited_by(limiter)` - Apply rate limiting with specified limiter

## Examples

### Database Connection Pool Rate Limiting

```rust
use rate_limiter::{TokenBucket, RateLimitedTask};

// Limit database connections
let db_limiter = TokenBucket::new()
   .bucket_size(20)        // 20 concurrent connections
   .refill_rate(5.0);      // Allow 5 new connections per second

async fn query_database(query: &str) -> Result<String, String> {
   // Simulate database query
   tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
   Ok(format!("Result for: {}", query))
}

// Rate limited database queries
let query1 = RateLimitedTask(query_database("SELECT * FROM users"))
   .limited_by(db_limiter.clone());
let query2 = RateLimitedTask(query_database("SELECT * FROM products"))
   .limited_by(db_limiter.clone());

let (result1, result2) = futures::join!(query1, query2);
```

### API Client with Multiple Rate Limits

```rust
use rate_limiter::{SlidingWindow, FixedWindow, RateLimitedTask};

// Different limits for different API endpoints
let search_limiter = SlidingWindow::new()
   .limit(1000)            // 1000 searches 
   .window_size(3600);     // per hour

let write_limiter = FixedWindow::new()
   .limit(100)             // 100 writes
   .window_size(60);       // per minute

async fn search_api(query: &str) -> String {
   format!("Search results for: {}", query)
}

async fn write_api(data: &str) -> String {
   format!("Wrote: {}", data)
}

// Apply appropriate rate limiting
let search = RateLimitedTask(search_api("rust"))
   .limited_by(search_limiter);
let write = RateLimitedTask(write_api("some data"))
   .limited_by(write_limiter);

let (search_result, write_result) = futures::join!(search, write);
```

## Testing

Run the test suite:

```bash
cargo test
```

Run tests with output:

```bash
cargo test -- --nocapture
```

## Performance

All rate limiters are designed to be efficient:
- Lock contention is minimized with fine-grained locking
- Memory usage is constant for Token and Leaky buckets
- Sliding window uses efficient cleanup of old requests
- No background threads or timers required

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Changelog

### v0.1.0
- Initial release
- Token Bucket, Leaky Bucket, Fixed Window, and Sliding Window algorithms
- Async/await support
- Thread-safe implementation
- RateLimitedTask wrapper for easy integration

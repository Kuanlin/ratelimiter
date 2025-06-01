use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tokio::time::sleep;

// Token Bucket 實現
#[derive(Debug, Clone)]
pub struct TokenBucket {
    state: Arc<Mutex<TokenBucketState>>,
}

#[derive(Debug)]
struct TokenBucketState {
    tokens: f64,
    max_tokens: u32,
    refill_rate: f64, // tokens per second
    last_refill: Instant,
}

impl TokenBucket {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(TokenBucketState {
                tokens: 0.0,
                max_tokens: 100,
                refill_rate: 1.0,
                last_refill: Instant::now(),
            })),
        }
    }

    pub fn max(self, max_tokens: u32) -> Self {
        {
            let mut state = self.state.lock().unwrap();
            state.max_tokens = max_tokens;
            state.tokens = state.tokens.min(max_tokens as f64);
        }
        self
    }

    pub fn refill_rate(self, rate: f64) -> Self {
        {
            let mut state = self.state.lock().unwrap();
            state.refill_rate = rate;
        }
        self
    }

    pub async fn acquire(&self) {
        loop {
            let delay = {
                let mut state = self.state.lock().unwrap();
                let now = Instant::now();
                let elapsed = now.duration_since(state.last_refill).as_secs_f64();
                
                // Refill tokens
                let new_tokens = elapsed * state.refill_rate;
                state.tokens = (state.tokens + new_tokens).min(state.max_tokens as f64);
                state.last_refill = now;

                if state.tokens >= 1.0 {
                    state.tokens -= 1.0;
                    None
                } else {
                    let wait_time = (1.0 - state.tokens) / state.refill_rate;
                    Some(Duration::from_secs_f64(wait_time))
                }
            };

            match delay {
                None => break,
                Some(delay) => sleep(delay).await,
            }
        }
    }
}

// Leaky Bucket 實現
#[derive(Debug, Clone)]
pub struct LeakyBucket {
    state: Arc<Mutex<LeakyBucketState>>,
}

#[derive(Debug)]
struct LeakyBucketState {
    queue_size: u32,
    max_size: u32,
    leak_rate: f64, // items per second
    last_leak: Instant,
}

impl LeakyBucket {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(LeakyBucketState {
                queue_size: 0,
                max_size: 100,
                leak_rate: 1.0,
                last_leak: Instant::now(),
            })),
        }
    }

    pub fn max(self, max_size: u32) -> Self {
        {
            let mut state = self.state.lock().unwrap();
            state.max_size = max_size;
        }
        self
    }

    pub fn leaky_rate(self, rate: f64) -> Self {
        {
            let mut state = self.state.lock().unwrap();
            state.leak_rate = rate;
        }
        self
    }

    pub async fn acquire(&self) {
        loop {
            let delay = {
                let mut state = self.state.lock().unwrap();
                let now = Instant::now();
                let elapsed = now.duration_since(state.last_leak).as_secs_f64();
                
                // Leak items
                let leaked_items = (elapsed * state.leak_rate) as u32;
                state.queue_size = state.queue_size.saturating_sub(leaked_items);
                state.last_leak = now;

                if state.queue_size < state.max_size {
                    state.queue_size += 1;
                    None
                } else {
                    let wait_time = 1.0 / state.leak_rate;
                    Some(Duration::from_secs_f64(wait_time))
                }
            };

            match delay {
                None => break,
                Some(delay) => sleep(delay).await,
            }
        }
    }
}

// Fixed Window 實現
#[derive(Debug, Clone)]
pub struct FixedWindow {
    state: Arc<Mutex<FixedWindowState>>,
}

#[derive(Debug)]
struct FixedWindowState {
    count: u32,
    max_count: u32,
    window_start: Instant,
    window_duration: Duration,
}

impl FixedWindow {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(FixedWindowState {
                count: 0,
                max_count: 100,
                window_start: Instant::now(),
                window_duration: Duration::from_secs(60),
            })),
        }
    }

    pub fn max(self, max_count: u32) -> Self {
        {
            let mut state = self.state.lock().unwrap();
            state.max_count = max_count;
        }
        self
    }

    pub fn duration(self, seconds: u64) -> Self {
        {
            let mut state = self.state.lock().unwrap();
            state.window_duration = Duration::from_secs(seconds);
        }
        self
    }

    pub async fn acquire(&self) {
        loop {
            let delay = {
                let mut state = self.state.lock().unwrap();
                let now = Instant::now();
                
                // Check if window has expired
                if now.duration_since(state.window_start) >= state.window_duration {
                    state.count = 0;
                    state.window_start = now;
                }

                if state.count < state.max_count {
                    state.count += 1;
                    None
                } else {
                    let window_end = state.window_start + state.window_duration;
                    Some(window_end.duration_since(now))
                }
            };

            match delay {
                None => break,
                Some(delay) => sleep(delay).await,
            }
        }
    }
}

// Sliding Window 實現
#[derive(Debug, Clone)]
pub struct SlidingWindow {
    state: Arc<Mutex<SlidingWindowState>>,
}

#[derive(Debug)]
struct SlidingWindowState {
    requests: VecDeque<Instant>,
    max_count: u32,
    window_duration: Duration,
}

impl SlidingWindow {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(SlidingWindowState {
                requests: VecDeque::new(),
                max_count: 100,
                window_duration: Duration::from_secs(60),
            })),
        }
    }

    pub fn max(self, max_count: u32) -> Self {
        {
            let mut state = self.state.lock().unwrap();
            state.max_count = max_count;
        }
        self
    }

    pub fn duration(self, seconds: u64) -> Self {
        {
            let mut state = self.state.lock().unwrap();
            state.window_duration = Duration::from_secs(seconds);
        }
        self
    }

    pub async fn acquire(&self) {
        loop {
            let delay = {
                let mut state = self.state.lock().unwrap();
                let now = Instant::now();
                let window_start = now - state.window_duration;
                
                // Remove old requests
                while let Some(&front_time) = state.requests.front() {
                    if front_time <= window_start {
                        state.requests.pop_front();
                    } else {
                        break;
                    }
                }

                if state.requests.len() < state.max_count as usize {
                    state.requests.push_back(now);
                    None
                } else {
                    // Wait until the oldest request expires
                    if let Some(&oldest) = state.requests.front() {
                        let wait_until = oldest + state.window_duration;
                        Some(wait_until.duration_since(now))
                    } else {
                        None
                    }
                }
            };

            match delay {
                None => break,
                Some(delay) => sleep(delay).await,
            }
        }
    }
}

// 枚舉來統一所有限流器類型
#[derive(Debug, Clone)]
pub enum AnyRateLimiter {
    TokenBucket(TokenBucket),
    LeakyBucket(LeakyBucket),
    FixedWindow(FixedWindow),
    SlidingWindow(SlidingWindow),
}

impl AnyRateLimiter {
    pub async fn acquire(&self) {
        match self {
            AnyRateLimiter::TokenBucket(limiter) => limiter.acquire().await,
            AnyRateLimiter::LeakyBucket(limiter) => limiter.acquire().await,
            AnyRateLimiter::FixedWindow(limiter) => limiter.acquire().await,
            AnyRateLimiter::SlidingWindow(limiter) => limiter.acquire().await,
        }
    }
}

impl From<TokenBucket> for AnyRateLimiter {
    fn from(limiter: TokenBucket) -> Self {
        AnyRateLimiter::TokenBucket(limiter)
    }
}

impl From<LeakyBucket> for AnyRateLimiter {
    fn from(limiter: LeakyBucket) -> Self {
        AnyRateLimiter::LeakyBucket(limiter)
    }
}

impl From<FixedWindow> for AnyRateLimiter {
    fn from(limiter: FixedWindow) -> Self {
        AnyRateLimiter::FixedWindow(limiter)
    }
}

impl From<SlidingWindow> for AnyRateLimiter {
    fn from(limiter: SlidingWindow) -> Self {
        AnyRateLimiter::SlidingWindow(limiter)
    }
}

// Rate Limited Task 包裝器
pub struct RateLimitedTask<F> {
    future: Option<F>,
    limiter: Option<AnyRateLimiter>,
    permit_acquired: bool,
}

impl<F> RateLimitedTask<F>
where
    F: Future,
{
    pub fn new(future: F) -> Self {
        Self {
            future: Some(future),
            limiter: None,
            permit_acquired: false,
        }
    }

    pub fn limited_by<L: Into<AnyRateLimiter>>(mut self, limiter: L) -> Self {
        self.limiter = Some(limiter.into());
        self
    }
}

impl<F> Future for RateLimitedTask<F>
where
    F: Future,
{
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        
        // 如果沒有限流器，直接執行
        if this.limiter.is_none() {
            if let Some(future) = this.future.as_mut() {
                let future = unsafe { Pin::new_unchecked(future) };
                return future.poll(cx);
            } else {
                panic!("Future already consumed");
            }
        }

        // 如果還沒獲得許可
        if !this.permit_acquired {
            if let Some(limiter) = &this.limiter {
                // 創建 acquire future 並立即 poll
                let mut acquire_future = Box::pin(limiter.acquire());
                match acquire_future.as_mut().poll(cx) {
                    Poll::Ready(()) => {
                        this.permit_acquired = true;
                        // 繼續執行原始 future
                    }
                    Poll::Pending => return Poll::Pending,
                }
            }
        }

        // 執行原始 future
        if let Some(future) = this.future.as_mut() {
            let future = unsafe { Pin::new_unchecked(future) };
            future.poll(cx)
        } else {
            panic!("Future already consumed");
        }
    }
}

// 便利的構造函數 - 保持大寫以符合你的使用習慣
#[allow(non_snake_case)]
pub fn RateLimitedTask<F>(future: F) -> RateLimitedTask<F>
where
    F: Future,
{
    RateLimitedTask::new(future)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn test_token_bucket() {
        let limiter = TokenBucket::new()
            .max(5)
            .refill_rate(10.0); // 10 tokens per second

        let start = Instant::now();
        for _ in 0..3 {
            limiter.acquire().await;
        }
        let elapsed = start.elapsed();
        
        // 前面幾個請求應該很快完成
        assert!(elapsed < Duration::from_millis(500));
    }

    #[tokio::test]
    async fn test_sliding_window() {
        let limiter = SlidingWindow::new()
            .max(2)
            .duration(1);

        // 應該可以立即獲得兩次許可
        limiter.acquire().await;
        limiter.acquire().await;

        // 第三次應該需要等待
        let start = Instant::now();
        limiter.acquire().await;
        let elapsed = start.elapsed();
        
        assert!(elapsed >= Duration::from_millis(900));
    }

    #[tokio::test]
    async fn test_rate_limited_task() {
        let limiter = FixedWindow::new()
            .max(1)
            .duration(1);

        async fn dummy_task() -> i32 {
            42
        }

        let task = RateLimitedTask(dummy_task()).limited_by(limiter);
        let result = task.await;
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_concurrent_tasks() {
        let limiter = SlidingWindow::new()
            .max(2)
            .duration(2);

        async fn dummy_request(id: i32) -> i32 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            id
        }

        let task1 = RateLimitedTask(dummy_request(1)).limited_by(limiter.clone());
        let task2 = RateLimitedTask(dummy_request(2)).limited_by(limiter.clone());

        let start = Instant::now();
        let (result1, result2) = futures::join!(task1, task2);
        let elapsed = start.elapsed();

        assert_eq!(result1, 1);
        assert_eq!(result2, 2);
        // 兩個任務應該能並發執行
        assert!(elapsed < Duration::from_millis(500));
    }
}

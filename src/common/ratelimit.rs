//! Token bucket rate limiter with per-IP tracking.

use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, Response, StatusCode},
    middleware::Next,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub burst_size: u32,
    pub requests_per_second: f64,
    pub window_duration: Duration,
    pub enabled: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            burst_size: 100,
            requests_per_second: 50.0,
            window_duration: Duration::from_secs(60),
            enabled: true,
        }
    }
}

#[derive(Debug, Clone)]
struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
    burst_size: u32,
    refill_rate: f64,
}

impl TokenBucket {
    fn new(burst_size: u32, refill_rate: f64) -> Self {
        Self {
            tokens: burst_size as f64,
            last_refill: Instant::now(),
            burst_size,
            refill_rate,
        }
    }

    /// Try to consume a token. Returns true if allowed, false if rate limited.
    fn try_consume(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.burst_size as f64);
        self.last_refill = now;
    }

    fn remaining(&self) -> u32 {
        self.tokens as u32
    }

    fn retry_after(&self) -> Duration {
        if self.tokens >= 1.0 {
            Duration::ZERO
        } else {
            let needed = 1.0 - self.tokens;
            Duration::from_secs_f64(needed / self.refill_rate)
        }
    }
}

#[derive(Clone)]
pub struct RateLimiter {

//! Prometheus-compatible metrics collection.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const LATENCY_BUCKETS: [f64; 11] = [
    1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0,
];

#[derive(Debug)]
pub struct Histogram {
    buckets: Vec<AtomicU64>,
    boundaries: Vec<f64>,
    sum: AtomicU64,
    count: AtomicU64,
}

impl Histogram {
    pub fn new() -> Self {
        Self::with_buckets(&LATENCY_BUCKETS)
    }

    pub fn with_buckets(boundaries: &[f64]) -> Self {
        let mut buckets = Vec::with_capacity(boundaries.len() + 1);
        for _ in 0..=boundaries.len() {
            buckets.push(AtomicU64::new(0));
        }
        Self {
            buckets,
            boundaries: boundaries.to_vec(),
            sum: AtomicU64::new(0),
            count: AtomicU64::new(0),
        }
    }

    pub fn observe(&self, value: f64) {
        let mut bucket_idx = self.boundaries.len();
        for (i, &boundary) in self.boundaries.iter().enumerate() {
            if value <= boundary {
                bucket_idx = i;
                break;
            }
        }

        self.buckets[bucket_idx].fetch_add(1, Ordering::Relaxed);
        self.sum
            .fetch_add((value * 1000.0) as u64, Ordering::Relaxed); // Store as microseconds for precision
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_buckets(&self) -> Vec<(f64, u64)> {
        let mut cumulative = 0u64;
        let mut result = Vec::with_capacity(self.boundaries.len() + 1);

        for (i, &boundary) in self.boundaries.iter().enumerate() {
            cumulative += self.buckets[i].load(Ordering::Relaxed);
            result.push((boundary, cumulative));
        }

        cumulative += self.buckets[self.boundaries.len()].load(Ordering::Relaxed);
        result.push((f64::INFINITY, cumulative));

        result
    }

    pub fn sum(&self) -> f64 {
        self.sum.load(Ordering::Relaxed) as f64 / 1000.0 // Convert back from microseconds
    }

    pub fn count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }
}

impl Default for Histogram {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Default)]
pub struct Counter {
    value: AtomicU64,
}

impl Counter {
    pub fn new() -> Self {
        Self {
            value: AtomicU64::new(0),
        }
    }

    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add(&self, n: u64) {
        self.value.fetch_add(n, Ordering::Relaxed);
    }

    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }
}

#[derive(Debug, Default)]
pub struct Gauge {
    value: AtomicU64,
}

impl Gauge {
    pub fn new() -> Self {
        Self {
            value: AtomicU64::new(0),
        }
    }

    pub fn set(&self, v: u64) {
        self.value.store(v, Ordering::Relaxed);
    }

    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    pub fn dec(&self) {
        self.value.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }
}

#[derive(Debug)]

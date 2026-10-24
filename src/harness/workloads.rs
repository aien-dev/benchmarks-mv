// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Drake Stapleton (AIEN / aien-dev)
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::time::Instant;

#[allow(dead_code)]
pub struct VectorBenchmarkResult {
    pub iterations: usize,
    pub dimensions: usize,
    pub elapsed_ms: f64,
    pub operations_per_sec: f64,
    pub p50_latency_us: f64,
}

pub fn run_vector_dot_product_benchmark(
    iterations: usize,
    dimensions: usize,
) -> VectorBenchmarkResult {
    let v1: Vec<f32> = (0..dimensions).map(|i| (i as f32) * 0.001).collect();
    let v2: Vec<f32> = (0..dimensions).map(|i| (i as f32) * 0.002).collect();

    let start = Instant::now();
    let mut accumulator = 0.0f32;

    for _ in 0..iterations {
        // Direct SIMD-friendly vector dot product
        let dot: f32 = v1.iter().zip(v2.iter()).map(|(a, b)| a * b).sum();
        accumulator += dot;
    }

    let elapsed = start.elapsed();
    let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
    let ops_per_sec = (iterations as f64) / elapsed.as_secs_f64().max(0.00001);
    let p50_us = (elapsed.as_micros() as f64) / (iterations as f64).max(1.0);

    // Prevent compiler optimization from dead-stripping computation
    if accumulator == 0.0000001 {
        println!("Edge case");
    }

    VectorBenchmarkResult {
        iterations,
        dimensions,
        elapsed_ms,
        operations_per_sec: ops_per_sec,
        p50_latency_us: p50_us,
    }
}

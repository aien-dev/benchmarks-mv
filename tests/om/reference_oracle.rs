// SPDX-License-Identifier: AGPL-3.0-or-later
// Test-only oracle. Production Omega binaries do not link or execute Rust.
#[allow(dead_code)]
#[path = "../../prime-drag-race/src/reference.rs"]
mod reference;
use std::io::{self, BufWriter, Write};

fn number(out: &mut impl Write, n: u64) -> io::Result<()> {
    out.write_all(&n.to_le_bytes())
}

fn bitmap(out: &mut impl Write, limit: u64) -> io::Result<()> {
    let bytes = reference::canonical_bytes(limit);
    number(out, reference::prime_count(&bytes, limit))?;
    out.write_all(&bytes)
}

fn next(x: &mut u64) -> u64 {
    *x = x
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *x
}

fn main() -> io::Result<()> {
    let mut out = BufWriter::new(io::stdout().lock());
    for n in 0..=100_000 {
        number(&mut out, u64::from(reference::is_prime(n)))?;
    }
    for n in 0..100 {
        number(&mut out, u64::from(reference::is_prime(u64::MAX - n)))?;
    }
    let mut x = 123456789;
    for _ in 0..10000 {
        let a = next(&mut x);
        let b = next(&mut x);
        let m = next(&mut x) | 1;
        number(&mut out, reference::mulmod(a, b, m))?;
        number(&mut out, reference::powmod(a, b & 255, m))?;
    }
    number(
        &mut out,
        reference::mulmod(u64::MAX, u64::MAX, u64::MAX - 1),
    )?;
    number(&mut out, reference::mulmod(u64::MAX, u64::MAX, 1))?;
    for limit in 0..=2000 {
        bitmap(&mut out, limit)?;
    }
    for limit in [100_000, 1_000_000, 10_000_000] {
        bitmap(&mut out, limit)?;
    }
    out.flush()
}

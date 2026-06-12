use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ca_he_core::{BitGrid, ReversibleCA};

fn bench_ca(c: &mut Criterion) {
    // Benchmark 1: Grid size 64 (1 u64 word)
    let size_64 = 64;
    let rule_lut = 30;
    let steps = 32;
    let prev_64 = BitGrid::from_u64(0x1234567890ABCDEF, size_64);
    let curr_64 = BitGrid::from_u64(0xFEDCBA0987654321, size_64);
    let ca_64 = ReversibleCA::new(rule_lut, steps);

    c.bench_function("evolve_64_cells_32_steps", |b| {
        b.iter(|| ca_64.evolve(black_box(&prev_64), black_box(&curr_64)))
    });

    // Benchmark 2: Grid size 1024 (16 u64 words)
    let size_1024 = 1024;
    let mut prev_1024 = BitGrid::new(size_1024);
    let mut curr_1024 = BitGrid::new(size_1024);
    for i in 0..size_1024 {
        if i % 3 == 0 { prev_1024.set_bit(i, true); }
        if i % 5 == 0 { curr_1024.set_bit(i, true); }
    }
    let ca_1024 = ReversibleCA::new(rule_lut, steps);

    c.bench_function("evolve_1024_cells_32_steps", |b| {
        b.iter(|| ca_1024.evolve(black_box(&prev_1024), black_box(&curr_1024)))
    });
}

criterion_group!(benches, bench_ca);
criterion_main!(benches);

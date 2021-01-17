use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use fcp::copy_file;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

fn baseline_copy_file(source: &str, dest: &str) {
    Command::new("cp")
        .arg("-R")
        .arg(source)
        .arg(dest)
        .output()
        .unwrap();
}

fn bench_copies(c: &mut Criterion) {
    let mut group = c.benchmark_group("Copy");
    let source = "example_source";
    let dest = "bench_source";
    let mut cleanup = Command::new("rm");
    cleanup.arg("-rf").arg(dest);
    group.warm_up_time(Duration::from_secs(20));
    group.measurement_time(Duration::from_secs(15 * 100));
    group.bench_with_input(
        BenchmarkId::new("Baseline", ""),
        &(source, dest),
        |b, (source, dest)| {
            b.iter_with_setup(|| cleanup.output(), |_| baseline_copy_file(source, dest))
        },
    );
    group.bench_with_input(
        BenchmarkId::new("FCP", ""),
        &(PathBuf::from(source), PathBuf::from(dest)),
        |b, (source, dest)| b.iter_with_setup(|| cleanup.output(), |_| copy_file(source, dest)),
    );
    group.finish();
}

criterion_group!(benches, bench_copies);
criterion_main!(benches);

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use dev_utils::*;
use std::process::Command;
use std::time::Duration;

fn baseline_copy_file(source: &str, dest: &str) {
    assert!(Command::new("cp")
        .args(&["-R", source, dest])
        .status()
        .unwrap()
        .success());
}

fn fcp_copy_file(source: &str, dest: &str, executable_path: &str) {
    assert!(Command::new(executable_path)
        .args(&[source, dest])
        .status()
        .unwrap()
        .success());
}

fn bench_copies(c: &mut Criterion) {
    initialize();
    let fixture_file = "linux.json";
    hydrate_fixture(fixture_file);
    let source_path = HYDRATED_DIR.join(fixture_file.strip_suffix(".json").unwrap());
    let dest_path = COPIES_DIR.join(fixture_file.strip_suffix(".json").unwrap());
    let (source, dest) = (source_path.to_str().unwrap(), dest_path.to_str().unwrap());
    remove(&dest_path);
    let executable_path = fcp_executable_path();
    let executable_path = executable_path.to_str().unwrap();
    let mut group = c.benchmark_group("Copy");
    group.warm_up_time(Duration::from_secs(30));
    group.measurement_time(Duration::from_secs(15 * 100));
    group.bench_with_input(
        BenchmarkId::new("Baseline", ""),
        &(source, dest),
        |b, (source, dest)| {
            b.iter_with_setup(|| remove(&dest_path), |_| baseline_copy_file(source, dest))
        },
    );
    group.bench_with_input(
        BenchmarkId::new("FCP", ""),
        &(source, dest),
        |b, (source, dest)| {
            b.iter_with_setup(
                || remove(&dest_path),
                |_| fcp_copy_file(source, dest, executable_path),
            )
        },
    );
    group.finish();
}

criterion_group!(benches, bench_copies);
criterion_main!(benches);

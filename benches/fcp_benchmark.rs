use criterion::{
    criterion_group, criterion_main, measurement::WallTime, BenchmarkGroup, BenchmarkId, Criterion,
    SamplingMode,
};
use dev_utils::*;
use std::process::Command;
use std::time::Duration;

fn run_command(mut command: Command) {
    assert!(command.status().unwrap().success());
}

fn fcp_benchmark(mut group: BenchmarkGroup<WallTime>, fixture_file: &str) {
    initialize();
    hydrate_fixture(fixture_file);
    let source_path = HYDRATED_DIR.join(fixture_file.strip_suffix(".json").unwrap());
    let dest_path = COPIES_DIR.join(fixture_file.strip_suffix(".json").unwrap());
    let (source, dest) = (source_path.to_str().unwrap(), dest_path.to_str().unwrap());
    remove(&dest_path);
    let executable_path = fcp_executable_path();
    let executable_path = executable_path.to_str().unwrap();
    group.bench_with_input(
        BenchmarkId::new("Baseline", ""),
        &(source, dest),
        |b, (source, dest)| {
            b.iter_with_setup(
                || {
                    remove(&dest_path);
                    let mut command = Command::new("cp");
                    command.args(&["-R", source, dest]);
                    command
                },
                run_command,
            )
        },
    );
    group.bench_with_input(
        BenchmarkId::new("FCP", ""),
        &(source, dest),
        |b, (source, dest)| {
            b.iter_with_setup(
                || {
                    remove(&dest_path);
                    let mut command = Command::new(executable_path);
                    command.args(&[source, dest]);
                    command
                },
                run_command,
            )
        },
    );
    group.finish();
}

fn linux_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Linux");
    group.sampling_mode(SamplingMode::Flat);
    group.warm_up_time(Duration::from_secs(60));
    group.sample_size(50);
    fcp_benchmark(group, "linux.json");
}

fn large_files_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Large Files");
    group.sampling_mode(SamplingMode::Flat);
    group.warm_up_time(Duration::from_secs(60));
    group.sample_size(100);
    fcp_benchmark(group, "large_files.json");
}

criterion_group!(benches, linux_benchmark, large_files_benchmark);
criterion_main!(benches);

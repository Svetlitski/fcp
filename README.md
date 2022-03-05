# fcp

[![CI status](https://github.com/Svetlitski/fcp/actions/workflows/continuous_integration.yml/badge.svg?branch=master)](https://github.com/Svetlitski/fcp/actions/workflows/continuous_integration.yml)
[![fcp crate](https://img.shields.io/crates/v/fcp.svg)](https://crates.io/crates/fcp)
[![Packaging status](https://repology.org/badge/tiny-repos/fcp-faster-cp.svg)](https://repology.org/project/fcp-faster-cp/versions)


`fcp` is a [significantly faster](#benchmarks) alternative to the classic Unix [`cp(1)`](https://man7.org/linux/man-pages/man1/cp.1.html) command.

`fcp` aims to handle the most common use-cases of `cp` with much higher performance.

`fcp` does _not_ aim to completely replace `cp` with its myriad options.

**Note**: `fcp` is optimized for systems with an SSD. On systems with a HDD, `fcp` may exhibit poor performance.

## Installation

Please note that `fcp` supports only Unix-like operating systems (e.g. Linux, macOS, etc.).

### Pre-built binaries

Pre-built binaries for some systems can be found under [this repository's releases](https://github.com/Svetlitski/fcp/releases).

### Via [`cargo`](https://github.com/rust-lang/cargo)

`fcp` requires Rust version 1.53.0 or newer. `fcp` can be installed using `cargo` by running the following:

```sh
cargo install fcp
```

### Arch Linux

`fcp` can be installed on Arch Linux via the [`fcp-bin` AUR](https://aur.archlinux.org/packages/fcp-bin/).

### NixOS

As of NixOS 21.11 `fcp` is included in the stable channel. For earlier
versions, `fcp` is available through `nixpkgs-unstable`. Assuming you've
already added the `nixpkgs-unstable` channel, `fcp` can be installed by running
the following:

```sh
nix-env -iA unstable.fcp
```

### macOS

`fcp` can be installed on macOS via [Homebrew](https://brew.sh/) by running the following:

```sh
brew install fcp
```

## Usage

Usage information can be found by running `fcp --help`, and has been reproduced below:

```
fcp 0.2.1

USAGE:
    fcp [OPTIONS] SOURCE DESTINATION_FILE
    Copy SOURCE to DESTINATION_FILE, overwriting DESTINATION_FILE if it exists

    fcp [OPTIONS] SOURCE ... DESTINATION_DIRECTORY
    Copy each SOURCE into DESTINATION_DIRECTORY

OPTIONS:
    -h, --help
            Output this usage information and exit.

    -V, --version
            Output version information and exit.
```

## Benchmarks

`fcp` doesn't just _claim_ to be faster than `cp`, it _is_ faster than `cp`. As different operating systems display
different performance characteristics, the same benchmarks were run on both macOS and Linux.

### macOS

The following benchmarks were run on a 2018 MacBook Pro<sup><a href="#footnote-1">1</a></sup> (2.9 GHz 6-Core Intel Core i9, 16 GiB RAM, SSD) with [APFS](https://developer.apple.com/documentation/foundation/file_system/about_apple_file_system) as the filesystem.

#### Large Files

The following shows the result of a benchmark which copies a directory containing 13 different 512 MB files using `cp` and `fcp`, with `fcp` being approximately **822x faster** on average (note the units of the axes for each plot)<sup><a href="#footnote-2">2</a></sup>:

![`fcp` is approximately 822x faster than `cp`, with `fcp`'s average time to copy being approximately 4.5 milliseconds, while `cp`'s average time to copy is approximately 3.7 seconds](https://user-images.githubusercontent.com/35482043/122131973-a3990080-cdff-11eb-92dc-3e0d5f47ac07.png)

#### Linux Kernel Source

The following shows the result of a benchmark which copies the source tree of the Linux kernel using `cp` and `fcp`, with `fcp` being approximately 6x faster on average:

![`fcp` is approximately 6x faster than `cp`, with `fcp`'s average time to copy being approximately 5.1 seconds, while `cp`'s average time to copy is approximately 30 seconds](https://user-images.githubusercontent.com/35482043/122131983-a7c51e00-cdff-11eb-8bbb-8c768998de56.png)

### Linux

The following benchmarks were run on a bare-metal AWS EC2 instance (a1.metal, 16 CPUs, 32 GiB RAM, SSD) with [XFS](https://en.wikipedia.org/wiki/XFS) as the filesystem.

#### Linux Kernel Source

The following shows the result of a benchmark which copies the source tree of the Linux kernel using `cp` and `fcp`, with `fcp` being approximately 10x faster on average:

![`fcp` is nearly 10x faster than `cp`, with `fcp`'s average time to copy being approximately 675 milliseconds, while `cp`'s average time to copy is approximately 6.02 seconds](https://user-images.githubusercontent.com/35482043/122125946-ae9b6300-cdf6-11eb-97dd-0e0bfb916ede.png)

#### Large Files

The following shows the result of a benchmark which copies a directory containing 13 different 512 MB files using `cp` and `fcp`, with `fcp` being approximately 1.4x faster on average:

![`fcp` is approximately 1.4x faster than `cp, with `fcp`'s average time to copy being approximately 8 seconds, while `cp`'s average time to copy is approximately 11.3 seconds](https://user-images.githubusercontent.com/35482043/122125941-ae02cc80-cdf6-11eb-9899-a93ed0442f6f.png)


## Methodology

`fcp`'s high-performance can be attributed to several factors, but is primarily
the result of leveraging parallelism, distributing the work of walking
directories and copying their contents across all of your machine's cores. This
leads to a significant performance increase on systems with an SSD, as more I/O
requests are issued over the same period of time (as compared to a
single-threaded approach), resulting in a higher-average queue depth, thus
allowing higher utilization of the SSD (as a function of its maximum IOPS) and
correspondingly higher throughput.

Additionally, on macOS (and perhaps in the future on other operating systems) `fcp`
utilizes the system's underlying [copy-on-write](https://en.wikipedia.org/wiki/Copy-on-write)
capability, dramatically reducing the time needed to copy large files.

These two factors – in addition to an overall performance-conscious approach to this problem – serve
to explain `fcp`'s significantly improved performance relative to `cp`.
<br>
<br>
<br>

<span id="footnote-1">[1]</span> While in general [you should avoid benchmarking on
laptops](https://lemire.me/blog/my-sayings/), `fcp` is a developer tool and
many developers work primarily on laptops. Also unlike with Linux where you can
rent by the second, the minimum tenancy for AWS EC2 macOS instances is 24
hours, and these benchmarks took less than an hour.

<span id="footnote-2">[2]</span> The massive difference in performance in this case is due
to `fcp` using `fclonefileat` and `fcopyfile` under the hood.

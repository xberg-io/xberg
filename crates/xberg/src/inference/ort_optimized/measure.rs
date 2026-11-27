//! Measurements of the saved-graph load, run explicitly; they check nothing.

use super::*;

/// Times checking published graphs of several sizes: a hashed check streams the file through
/// SHA-256, a stamped one only opens and stats it; `read` is a plain `fs::read` for scale. Run in
/// release:
/// `cargo test -p xberg --release --lib -- --ignored --nocapture graph_verification_cost`.
/// `XBERG_ORT_GRAPH_BENCH_MIB` overrides the sizes (comma-separated MiB, default `16,169,512`).
#[test]
#[ignore = "measurement, not a check; run explicitly in release"]
fn graph_verification_cost() {
    const RUNS: usize = 7;
    let sizes: Vec<usize> = std::env::var("XBERG_ORT_GRAPH_BENCH_MIB")
        .unwrap_or_else(|_| "16,169,512".to_string())
        .split(',')
        .map(|mib| mib.trim().parse().expect("sizes are integers in MiB"))
        .collect();
    let millis = |mut runs: Vec<Duration>| {
        runs.sort();
        (runs[0].as_secs_f64() * 1e3, runs[runs.len() / 2].as_secs_f64() * 1e3)
    };
    println!("size_mib\tread_min_ms\tread_med_ms\thashed_min_ms\thashed_med_ms\tstamped_min_ms\tstamped_med_ms");
    for mib in sizes {
        let root = TempDir::new().unwrap();
        let mut state = 0x9e37_79b9_7f4a_7c15_u64;
        let bytes: Vec<u8> = (0..mib << 20)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                state as u8
            })
            .collect();
        let hashing = Slot {
            settle: Duration::MAX,
            ..slot(&root)
        };
        let stamping = Slot::new(
            root.path().join("cache"),
            &root.path().join("stamped.onnx"),
            &"a".repeat(64),
            "recipe v1",
            &platform(),
        );
        let stamping = Slot {
            settle: Duration::ZERO,
            ..stamping
        };
        let path = save(&hashing, &bytes);
        save(&stamping, &bytes);

        let time = |load: &dyn Fn()| {
            load();
            (0..RUNS)
                .map(|_| {
                    let start = std::time::Instant::now();
                    load();
                    start.elapsed()
                })
                .collect::<Vec<_>>()
        };
        let read = millis(time(&|| assert_eq!(fs::read(&path).unwrap().len(), bytes.len())));
        let hashed = millis(time(&|| assert!(hashing.verified_graph().unwrap().unwrap().hashed)));
        let stamped = millis(time(&|| assert!(!stamping.verified_graph().unwrap().unwrap().hashed)));
        println!(
            "{mib}\t{:.1}\t{:.1}\t{:.1}\t{:.1}\t{:.1}\t{:.1}",
            read.0, read.1, hashed.0, hashed.1, stamped.0, stamped.1
        );
    }
}

const RSS_CHILD: &str = "XBERG_ORT_GRAPH_RSS_CHILD";
const RSS_FILE: &str = "XBERG_ORT_GRAPH_RSS_FILE";

/// The process's peak resident set size in bytes.
#[allow(unsafe_code)]
fn max_rss() -> u64 {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
    // SAFETY: `usage` is writable storage for one rusage, filled on success.
    assert_eq!(unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) }, 0);
    // SAFETY: getrusage succeeded, so `usage` is initialised.
    let max = u64::try_from(unsafe { usage.assume_init() }.ru_maxrss).unwrap_or(0);
    // macOS reports bytes, Linux kibibytes.
    if cfg!(target_os = "macos") { max } else { max * 1024 }
}

/// Peak RSS added by loading one saved graph three ways, each in a fresh process: by path, from
/// memory (`fs::read` + `commit_from_memory`) and through the checked descriptor
/// ([`Graph::commit`]). Run in release:
/// `cargo test -p xberg --release --lib -- --ignored --nocapture graph_load_peak_rss`.
/// `XBERG_ORT_GRAPH_RSS_MIB` sets the graph size (default 169).
#[test]
#[ignore = "measurement, not a check; run explicitly in release"]
fn graph_load_peak_rss() {
    const TEST: &str = "inference::ort_optimized::tests::measure::graph_load_peak_rss";
    if let Ok(mode) = std::env::var(RSS_CHILD) {
        if crate::ort_discovery::test_ort_environment("graph_load_peak_rss").is_none() {
            return;
        }
        let file = PathBuf::from(std::env::var(RSS_FILE).unwrap());
        let accel = cpu();
        let mut builder = with_level(
            crate::inference::ort_backend::session_builder(Some(&accel), 1, true).unwrap(),
            GraphOptimizationLevel::Disable,
        )
        .unwrap();
        let before = max_rss();
        let start = std::time::Instant::now();
        let session = match mode.as_str() {
            "path" => builder.commit_from_file(&file).unwrap(),
            "memory" => builder.commit_from_memory(&fs::read(&file).unwrap()).unwrap(),
            "fd" => {
                let handle = open_entry(&file).unwrap();
                let metadata = handle.metadata().unwrap();
                let graph = Graph {
                    path: file.clone(),
                    size: metadata.len(),
                    stamp: Stamp::of(&metadata),
                    file: handle,
                    hashed: false,
                };
                assert!(graph.descriptor_path().unwrap().is_some(), "measure the /dev/fd route");
                graph.commit(builder).unwrap()
            }
            other => panic!("unknown mode {other}"),
        };
        let elapsed = start.elapsed();
        let added = max_rss().saturating_sub(before);
        println!(
            "RSS\t{mode}\t{:.1}\t{:.1}",
            added as f64 / 1_048_576.0,
            elapsed.as_secs_f64() * 1e3
        );
        drop(session);
        return;
    }

    let mib: usize = std::env::var("XBERG_ORT_GRAPH_RSS_MIB").map_or(169, |mib| mib.parse().unwrap());
    let root = TempDir::new().unwrap();
    let file = root.path().join("large.onnx");
    fs::write(&file, tiny_model::large((mib << 20) / 4)).unwrap();
    println!("mode\tpeak_rss_added_mib\tload_ms");
    for mode in ["path", "memory", "fd"] {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--ignored", "--nocapture"])
            .env(RSS_CHILD, mode)
            .env(RSS_FILE, &file)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let row = stdout.lines().find_map(|line| line.strip_prefix("RSS\t"));
        assert!(
            output.status.success() && row.is_some(),
            "{mode}: stdout={stdout} stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        println!("{}", row.unwrap_or_default());
    }
}

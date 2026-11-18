use std::process::Command;
use std::sync::{Arc, Barrier};

const CHILD_ENV: &str = "XBERG_SCEPTRE_ORT_CHILD";
const CONCURRENT_CALLERS: usize = 4;
const EXPECTED_OUTPUT: f32 = 42.0;

fn isolated(name: &str, run: impl FnOnce()) {
    if std::env::var(CHILD_ENV).as_deref() == Ok(name) {
        run();
        return;
    }
    let test = format!("ort_discovery::sceptre_tests::{name}");
    let output = Command::new(std::env::current_exe().expect("locate unit test binary"))
        .args(["--exact", &test, "--nocapture"])
        .env(CHILD_ENV, name)
        .output()
        .expect("run isolated ORT initialization test");
    assert!(
        output.status.success(),
        "isolated test failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"));
}

fn infer(sceptre_first: bool) {
    let environment = if sceptre_first {
        sceptre::ort_environment().expect("initialize Sceptre environment")
    } else {
        super::ort_environment().expect("initialize Xberg environment")
    };
    let mut session = ort::session::Session::builder(environment)
        .expect("create native session options")
        .with_intra_threads(1)
        .expect("limit native inference threads")
        .commit_from_memory(&super::shared_environment_tests::identity_model())
        .expect("load a real ONNX model");
    let tensor = ort::value::Tensor::from_array(([1usize], vec![EXPECTED_OUTPUT])).expect("create tensor");
    let outputs = session
        .run(ort::inputs!["x" => tensor])
        .expect("run native model inference");
    let (shape, values) = outputs["y"]
        .try_extract_tensor::<f32>()
        .expect("extract inference result");
    assert_eq!(&shape[..], &[1]);
    assert_eq!(values, &[EXPECTED_OUTPUT]);
}

fn assert_shared_environment() {
    let xberg = super::ort_environment().expect("initialize Xberg environment");
    let sceptre = sceptre::ort_environment().expect("initialize Sceptre environment");
    assert!(std::ptr::eq(xberg, sceptre));
}

#[test]
fn should_run_sceptre_after_xberg_initialization() {
    isolated("should_run_sceptre_after_xberg_initialization", || {
        infer(false);
        infer(true);
        infer(false);
        assert_shared_environment();
    });
}

#[test]
fn should_run_xberg_after_sceptre_initialization() {
    isolated("should_run_xberg_after_sceptre_initialization", || {
        infer(true);
        infer(false);
        infer(true);
        assert_shared_environment();
    });
}

#[test]
fn should_run_concurrent_xberg_and_sceptre_inference() {
    isolated("should_run_concurrent_xberg_and_sceptre_inference", || {
        let barrier = Arc::new(Barrier::new(CONCURRENT_CALLERS));
        let callers: Vec<_> = (0..CONCURRENT_CALLERS)
            .map(|index| {
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    infer(index % 2 == 0)
                })
            })
            .collect();
        let completed: Vec<_> = callers
            .into_iter()
            .map(|caller| caller.join().expect("consumer must finish inference"))
            .collect();
        assert_eq!(completed.len(), CONCURRENT_CALLERS);
        assert_shared_environment();
    });
}

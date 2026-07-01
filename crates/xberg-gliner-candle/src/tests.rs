use candle_core::Device;

#[test]
fn encoder_from_safetensors_rejects_missing_weights() {
    let dir = tempfile::tempdir().expect("tempdir");
    let weights = dir.path().join("model.safetensors");
    let config = dir.path().join("config.json");
    let result = crate::encoder::Encoder::from_safetensors(&weights, &config, &Device::Cpu);
    match result {
        Ok(_) => panic!("missing files must error, not panic"),
        Err(e) => {
            let msg = e.to_string();
            assert!(
                msg.contains("encoder config read") || msg.contains("backend error"),
                "unexpected error: {msg}"
            );
        }
    }
}

#[test]
fn token_gather_selects_word_start_positions() {
    use candle_core::{Device, Tensor};
    let device = Device::Cpu;
    // hidden_states: [1, 3, 2] — 3 tokens, hidden size 2.
    let hidden = Tensor::from_vec(vec![1f32, 1., 2., 2., 3., 3.], (1, 3, 2), &device).unwrap();
    let word_indices = Tensor::from_vec(vec![0u32, 2u32], (2,), &device).unwrap();
    let out = crate::heads::token_gather::TokenGather
        .forward(&hidden, &word_indices)
        .unwrap();
    assert_eq!(out.dims(), &[1, 2, 2]);
    let v = out.flatten_all().unwrap().to_vec1::<f32>().unwrap();
    assert_eq!(v, vec![1., 1., 3., 3.]);
}

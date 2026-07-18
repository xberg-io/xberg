#[cfg(not(target_arch = "wasm32"))]
#[test]
fn encoder_from_safetensors_rejects_missing_weights() {
    use candle_core::Device;
    let dir = tempfile::tempdir().expect("tempdir");
    let weights = dir.path().join("model.safetensors");
    let config = dir.path().join("config.json");
    let result = crate::candle::encoder::Encoder::from_safetensors(&weights, &config, &Device::Cpu);
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
    // hidden_states: [1, 3, 2]; 3 tokens, hidden size 2.
    let hidden = Tensor::from_vec(vec![1f32, 1., 2., 2., 3., 3.], (1, 3, 2), &device).unwrap();
    let word_indices = Tensor::from_vec(vec![0u32, 2u32], (2,), &device).unwrap();
    let out = crate::candle::heads::token_gather::TokenGather
        .forward(&hidden, &word_indices)
        .unwrap();
    assert_eq!(out.dims(), &[1, 2, 2]);
    let v = out.flatten_all().unwrap().to_vec1::<f32>().unwrap();
    assert_eq!(v, vec![1., 1., 3., 3.]);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn from_local_rejects_missing_weights() {
    let dir = tempfile::tempdir().expect("tempdir");
    let err = crate::candle::Gliner2Candle::from_local(dir.path()).expect_err("empty dir must fail");
    assert!(
        err.to_string().contains("model.safetensors"),
        "error must mention 'model.safetensors', got: {err}"
    );
}

#[test]
fn count_pred_clamps_argmax_to_19() {
    use candle_core::{Device, Tensor};
    use candle_nn::VarBuilder;
    let device = Device::Cpu;
    let varmap = candle_nn::VarMap::new();
    let vb = VarBuilder::from_varmap(&varmap, candle_core::DType::F32, &device);
    let head = crate::candle::heads::count_pred::CountPred::from_var_builder(&vb.pp("count_pred"))
        .expect("zero-initialised weights still build a valid head");
    let p_emb = Tensor::zeros((1, 768), candle_core::DType::F32, &device).unwrap();
    let pred = head.forward(&p_emb).expect("forward must not panic on zero weights");
    assert!(pred < 20, "argmax must be clamped to [0, 19], got {pred}");
}

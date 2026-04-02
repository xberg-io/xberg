use crate::config::parse_config_embedding;
use crate::error::kreuzberg_error;
use extendr_api::prelude::*;

pub fn embed_impl(texts: Strings, config_json: Nullable<&str>) -> extendr_api::Result<List> {
    let config = parse_config_embedding(config_json)?;
    let texts_vec: Vec<String> = texts.into_iter().collect();
    
    let result = kreuzberg::embed_texts(&texts_vec, &config).map_err(kreuzberg_error)?;
    
    let list = result.into_iter()
        .map(|v| v.into_iter().map(|f| f as f64).collect::<Doubles>().into_robj())
        .collect::<List>();
        
    Ok(list)
}

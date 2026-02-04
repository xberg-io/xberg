use ort::session::{
    Session,
    builder::{GraphOptimizationLevel, SessionBuilder},
};

use crate::ocr_error::OcrError;

pub trait BaseNet {
    fn new() -> Self;

    fn get_session_builder(
        &self,
        num_thread: usize,
        builder_fn: Option<fn(SessionBuilder) -> Result<SessionBuilder, ort::Error>>,
    ) -> Result<SessionBuilder, OcrError> {
        let builder = Session::builder()?;
        let builder = match builder_fn {
            Some(custom) => custom(builder)?,
            None => builder
                .with_optimization_level(GraphOptimizationLevel::Level2)?
                .with_intra_threads(num_thread)?
                .with_inter_threads(num_thread)?,
        };

        Ok(builder)
    }

    fn set_input_names(&mut self, input_names: Vec<String>);
    fn set_session(&mut self, session: Option<Session>);

    fn init(&mut self, session: Session) {
        let input_names: Vec<String> = session.inputs().iter().map(|input| input.name().to_string()).collect();

        self.set_input_names(input_names);
        self.set_session(Some(session));
    }

    fn init_model(
        &mut self,
        path: &str,
        num_thread: usize,
        builder_fn: Option<fn(SessionBuilder) -> Result<SessionBuilder, ort::Error>>,
    ) -> Result<(), OcrError> {
        let session = self
            .get_session_builder(num_thread, builder_fn)?
            .commit_from_file(path)?;
        self.init(session);

        Ok(())
    }

    fn init_model_from_memory(
        &mut self,
        model_bytes: &[u8],
        num_thread: usize,
        builder_fn: Option<fn(SessionBuilder) -> Result<SessionBuilder, ort::Error>>,
    ) -> Result<(), OcrError> {
        let session = self
            .get_session_builder(num_thread, builder_fn)?
            .commit_from_memory(model_bytes)?;

        self.init(session);

        Ok(())
    }
}

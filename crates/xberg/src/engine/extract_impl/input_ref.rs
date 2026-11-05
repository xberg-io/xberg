use crate::core::config::{ExtractInput, ExtractInputWithOptions};

pub(in crate::engine) trait BatchInputRef: Send + 'static {
    fn input(&self) -> &ExtractInput;
    fn into_input(self) -> ExtractInput;
    fn use_layout_for_markdown(&self) -> Option<bool>;
}

impl BatchInputRef for ExtractInput {
    fn input(&self) -> &ExtractInput {
        self
    }

    fn into_input(self) -> ExtractInput {
        self
    }

    fn use_layout_for_markdown(&self) -> Option<bool> {
        None
    }
}

impl BatchInputRef for ExtractInputWithOptions {
    fn input(&self) -> &ExtractInput {
        &self.input
    }

    fn into_input(self) -> ExtractInput {
        self.input
    }

    fn use_layout_for_markdown(&self) -> Option<bool> {
        self.use_layout_for_markdown
    }
}

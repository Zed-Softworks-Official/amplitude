use crate::core::channels::{Channel, Send};

pub trait AudioBackend {
    fn create_channel(
        &mut self,
        name: String,
        default_sends: Vec<Send>,
    ) -> Result<Channel, String>;
}

use crate::core::channels::Channel;

pub trait AudioBackend {
    fn create_channel(
        &mut self,
        name: String,
    ) -> Result<Channel, String>;
}

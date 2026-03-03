use crate::core::channels::Channel;

#[derive(Debug, Clone)]
pub struct Config {
    pub channels: Vec<Channel>
}

use std::collections::HashMap;
use pw::proxy::ProxyT;

pub struct PwSinkManager {
    sinks: HashMap<String, SinkInfo>
}

#[derive(Debug)]
struct SinkInfo {
    node_id: u32,
    node_name: String,
}

impl PwSinkManager {
    pub fn new() -> PwSinkManager {
        Self {
            sinks: HashMap::new()
        }
    }

    pub fn create_virtual_sink(
        &mut self,
        core: &pw::core::CoreRc,
        channel_id: String,
        name: String
    ) -> Result<u32, Box<dyn std::error::Error>> {
        let node_name = format!("amplitude_channel_{}", channel_id);

        let props = pw::__properties__! {
            *pw::keys::FACTORY_NAME => "support.null-audio-sink",
            *pw::keys::NODE_NAME => node_name.as_str(),
            *pw::keys::NODE_DESCRIPTION => name.as_str(),
            *pw::keys::MEDIA_CLASS => "Audio/Sink",
        };

        let proxy = core.create_object::<pw::node::Node>(
            "adapter",
            &props,
        )?;

        let node_id = proxy.upcast_ref().id();

        let _listener = proxy
            .add_listener_local()
            .info(move |info| {
                log::info!("Sink created: {:?}", info);
            })
            .register();

        self.sinks.insert(
            channel_id,
            SinkInfo {
                node_name,
                node_id
            }
        );

        Ok(node_id)
    }
}

#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub id: u32,
    pub name: String,
    pub description: Option<String>,
    pub app_name: Option<String>,
    pub app_binary: Option<String>,
    pub media_class: Option<String>,
    pub icon: Option<String>,
    pub is_amplitude_virtual: bool,
}

use crate::OutputProtocol;

#[derive(Debug, Default)]
pub struct StandardErrorProtocol;

impl OutputProtocol for StandardErrorProtocol {
    async fn init(&self) {
        // No initialization needed
    }

    async fn send(self: std::sync::Arc<Self>, str: &str) {
        eprintln!("{}", str);
    }
}

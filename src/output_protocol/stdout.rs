use crate::OutputProtocol;

#[derive(Debug, Default)]
pub struct StandardOutputProtocol;

impl OutputProtocol for StandardOutputProtocol {
    async fn init(self) {
        // No initialization needed
    }

    async fn send(self: std::sync::Arc<Self>, str: &str) {
        println!("{}", str);
    }
}

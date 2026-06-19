use crate::OutputProtocol;

#[derive(Debug, Default)]
pub struct IPCOutputProtocol {}

impl OutputProtocol for IPCOutputProtocol {
    async fn init(self) {
        todo!()
    }

    async fn send(self: std::sync::Arc<Self>, _str: &str) {
        todo!()
    }
}

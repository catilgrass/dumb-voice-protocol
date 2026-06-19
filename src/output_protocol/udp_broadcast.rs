use crate::OutputProtocol;

#[derive(Debug, Default)]
pub struct UDPBroadcastOutputProtocol {}

impl OutputProtocol for UDPBroadcastOutputProtocol {
    async fn init(self) {
        todo!()
    }

    async fn send(self: std::sync::Arc<Self>, _str: &str) {
        todo!()
    }
}

use crate::OutputProtocol;

#[derive(Debug, Default)]
pub struct UDPOutputProtocol {}

impl OutputProtocol for UDPOutputProtocol {
    async fn init(self) {
        todo!()
    }

    async fn send(self: std::sync::Arc<Self>, _str: &str) {
        todo!()
    }
}

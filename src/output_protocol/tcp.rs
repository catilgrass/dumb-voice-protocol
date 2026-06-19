use crate::OutputProtocol;

#[derive(Debug, Default)]
pub struct TCPOutputProtocol {}

impl OutputProtocol for TCPOutputProtocol {
    async fn init(self) {
        todo!()
    }

    async fn send(self: std::sync::Arc<Self>, _str: &str) {
        todo!()
    }
}

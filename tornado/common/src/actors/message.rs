use actix::prelude::Message;
use failure_derive::Fail;
use tokio::prelude::AsyncRead;

#[derive(Fail, Debug)]
pub enum TornadoCommonActorError {
    #[fail(display = "ServerNotAvailableError: cannot connect to server [{}]", address)]
    ServerNotAvailableError { address: String },
    #[fail(display = "SerdeError: [{}]", message)]
    SerdeError { message: String },
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct StringMessage {
    pub msg: String,
}

#[derive(Message)]
#[rtype(result = "Result<(), TornadoCommonActorError>")]
pub struct EventMessage {
    pub event: tornado_common_api::Event,
}

#[derive(Message)]
#[rtype(result = "Result<(), TornadoCommonActorError>")]
pub struct BytesMessage {
    pub msg: Vec<u8>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct AsyncReadMessage<R: AsyncRead> {
    pub stream: R,
}

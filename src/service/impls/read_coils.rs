
use crate::service::traits::Service;
use crate::service::types::{AddressRange, Indexed};
use crate::session::UnitIdentifier;
use crate::channel::{Request, ServiceRequest};
use crate::error::Error;

use tokio::sync::oneshot;

impl Service for crate::service::services::ReadCoils {

    const REQUEST_FUNCTION_CODE: u8 = crate::function::constants::READ_COILS;

    type Request = AddressRange;
    type Response = Vec<Indexed<bool>>;

    fn create_request(unit_id: UnitIdentifier, argument: Self::Request, reply_to: oneshot::Sender<Result<Self::Response, Error>>) -> Request {
        Request::ReadCoils(ServiceRequest::new(unit_id, argument, reply_to))
    }
}

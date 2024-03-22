use std::fmt::Display;

use crate::client::message::Promise;
use crate::common::function::FunctionCode;
use crate::decode::AppDecodeLevel;
use crate::error::RequestError;
use crate::types::MutableFunctionCode;

use scursor::{ReadCursor, WriteCursor};

pub(crate) trait SendMutableFCOperation: Sized + PartialEq {
    fn serialize(&self, cursor: &mut WriteCursor) -> Result<(), RequestError>;
    fn parse(cursor: &mut ReadCursor) -> Result<Self, RequestError>;
}

pub(crate) struct SendMutableFC<T>
where
    T: SendMutableFCOperation + Display + Send + 'static,
{
    pub(crate) request: T,
    promise: Promise<T>,
}

impl<T> SendMutableFC<T>
where
    T: SendMutableFCOperation + Display + Send + 'static,
{
    pub(crate) fn new(request: T, promise: Promise<T>) -> Self {
        Self { request, promise }
    }

    pub(crate) fn serialize(&self, cursor: &mut WriteCursor) -> Result<(), RequestError> {
        self.request.serialize(cursor)
    }

    pub(crate) fn failure(&mut self, err: RequestError) {
        self.promise.failure(err)
    }

    pub(crate) fn handle_response(
        &mut self,
        cursor: ReadCursor,
        function: FunctionCode,
        decode: AppDecodeLevel,
    ) -> Result<(), RequestError> {
        let response = self.parse_all(cursor)?;

        if decode.data_headers() {
            tracing::info!("PDU RX - {} {}", function, response);
        } else if decode.header() {
            tracing::info!("PDU RX - {}", function);
        }

        self.promise.success(response);
        Ok(())
    }

    fn parse_all(&self, mut cursor: ReadCursor) -> Result<T, RequestError> {
        let response = T::parse(&mut cursor)?;
        cursor.expect_empty()?;
        Ok(response)
    }
}


impl SendMutableFCOperation for MutableFunctionCode {
    fn serialize(&self, cursor: &mut WriteCursor) -> Result<(), RequestError> {
        cursor.write_u8(self.function_code())?;
        for element in self.data() {
            cursor.write_u8(*element)?;
        }
        Ok(())
    }

    fn parse(cursor: &mut ReadCursor) -> Result<Self, RequestError> {
        let fc = cursor.read_u8()?;
        let raw_values = cursor.read_all();
        Ok(MutableFunctionCode::new(fc, raw_values.to_vec()))
    }
}

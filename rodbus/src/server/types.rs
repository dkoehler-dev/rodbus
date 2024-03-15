use crate::types::{AddressRange, BitIterator, RegisterIterator};

/// Request to write coils received by the server
#[derive(Debug, Copy, Clone)]
pub struct WriteCoils<'a> {
    /// address range of the request
    pub range: AddressRange,
    /// lazy iterator over the coil values to write
    pub iterator: BitIterator<'a>,
}

impl<'a> WriteCoils<'a> {
    pub(crate) fn new(range: AddressRange, iterator: BitIterator<'a>) -> Self {
        Self { range, iterator }
    }
}

/// Request to write registers received by the server
#[derive(Debug, Copy, Clone)]
pub struct WriteRegisters<'a> {
    /// address range of the request
    pub range: AddressRange,
    /// lazy iterator over the register values to write
    pub iterator: RegisterIterator<'a>,
}

impl<'a> WriteRegisters<'a> {
    pub(crate) fn new(range: AddressRange, iterator: RegisterIterator<'a>) -> Self {
        Self { range, iterator }
    }
}

/// Request to process a generic mutable function code
#[derive(Debug, Copy, Clone)]
pub struct MutableFunctionCode<'a> {
    /// function code to process
    pub function_code: u8,
    /// raw data of the request
    pub data: &'a [u8],
}

/*impl<'a> MutableFunctionCode<'a> {
    pub(crate) fn new(function_code: u8, data: &'a [u8]) -> Self {
        Self { function_code, data }
    }
}*/
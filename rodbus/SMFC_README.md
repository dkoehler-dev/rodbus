# Send Mutable Function Code (0x00)

This document provides a detailed overview of the implemented rodbus mutable function code feature. This function code is used as a generic wrapper around a standard modbus request. The client-side is wrapping the request as a generic mutable FC request and the server-side device is unwrapping the request and processes the underlying modbus request, returning the specific (unwrapped) response.


## Introduction
The generic mutable function code acts as a general interface, enabling the user to call each modbus function code (0-255) in a generic and streamlined way.


## Request Structure
| Parameter           | Size          | Range / Value         |
|---------------------|---------------|-----------------------|
| Function code       | 1 Byte        | 0x00                  |
| Function code       | 1 Byte        | 0x01 to 0xFF          |
| Data                | N* x M* Bytes | 0x0000 to 0xFFFF      |
N* & M* - The supplied data depends on the specific underlying function code that is called

## Response Structure
| Parameter           | Size          | Value/Description     |
|---------------------|---------------|-----------------------|
| Function code       | 1 Byte        | 0x01 to 0xFF          |
| Data                | N* x M* Bytes | 0x0000 to 0xFFFF      |
N* & M* - The supplied data depends on the specific underlying function code that is called


## Error Handling
| Parameter      | Size    | Description          |
|----------------|---------|----------------------|
| Function code  | 1 Byte  | Function code + 0x80 |
| Exception code | 1 Byte  | 01 or 02 or 03 or 04 |

Note: If the FC + 80 is expected to be above 255 (>175), the server can process the request but if it throws an error, it can only respond with a generic exception instead of the below described Exception Codes.

### Exception Codes:
- **01**: Illegal Function
- **02**: Illegal Data Address
- **03**: Illegal Data Value
- **04**: Server Device Failure


## Usage Example
### Request to send the mutable FC 0x01 (read coils) with a data of [0, 0, 0, 5] (start at address 0, read 5 coils):
| Request Field             | Hex | Response Field         | Hex |
|---------------------------|-----|------------------------|-----|
| Function code             | 00  | Function code          | 01  |
| Function code             | 01  | Byte count             | 01  |
| Starting Address Hi       | 00  | Outputs status 0-5     | 00  |
| Starting Address Lo       | 00  |                        |     |
| Quantity of coils Hi      | 00  |                        |     |
| Quantity of coils Lo      | 05  |                        |     |


## Usage
Make sure that you are in the `rodbus` project directory.


### Start the mutable_server example
- `cargo run --example mutable_server -- tcp`
- Once it's up, run `ed` to enable decoding

Leave the terminal open and open another terminal.


### Start the mutable_client example
- `cargo run --example mutable_client -- tcp`
- Once it's up, run `ed` to enable decoding as well


### Send the Mutable Function Code 0x01
In the terminal with the running mutable_client example, run:
- `smfc <u8 Function Code>` for a quick request with example data
- E.g. `smfc 0x01`
- `smfc <u8 Function Code> <u8 or u16 data>` to provide your own data
- E.g. `smfc 0x01 0x00 0x00 0x00 0x05`
- A possible response would would look like this for example: `fc: 0x1, data: [0x0]`


## Troubleshooting Tips
- Ensure the server and client are using the same communication method and are connected to each other.
- Check for any error codes in the response and refer to the error handling section for resolution.


## Additional Resources
- For more information on the MODBUS protocol and function codes, refer to the [MODBUS Application Protocol Specification V1.1b3](https://modbus.org/docs/Modbus_Application_Protocol_V1_1b3.pdf).
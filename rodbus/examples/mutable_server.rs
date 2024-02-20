use std::process::exit;
use std::sync::{Arc, Mutex};

use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};

use rodbus::server::*;
use rodbus::*;

struct SimpleHandler {
    coils: Arc<Mutex<Vec<bool>>>,
    discrete_inputs: Arc<Mutex<Vec<bool>>>,
    holding_registers: Arc<Mutex<Vec<u16>>>,
    input_registers: Arc<Mutex<Vec<u16>>>,
}

impl SimpleHandler {
    fn new(
        coils: Vec<bool>,
        discrete_inputs: Vec<bool>,
        holding_registers: Vec<u16>,
        input_registers: Vec<u16>,
    ) -> Self {
        Self {
            coils: Arc::new(Mutex::new(coils)),
            discrete_inputs: Arc::new(Mutex::new(discrete_inputs)),
            holding_registers: Arc::new(Mutex::new(holding_registers)),
            input_registers: Arc::new(Mutex::new(input_registers)),
        }
    }
}

// ANCHOR: request_handler
impl RequestHandler for SimpleHandler {
    fn read_coil(&self, address: u16) -> Result<bool, ExceptionCode> {
        //self.coils.get(address as usize).to_result()

        // Lock the mutex to prevent concurrent access
        let coils_lock = self.coils.lock().unwrap();
        // Return the value at the given address
        coils_lock.get(address as usize)
            .cloned()
            .ok_or(ExceptionCode::IllegalDataAddress)
    }

    fn read_discrete_input(&self, address: u16) -> Result<bool, ExceptionCode> {
        //self.discrete_inputs.get(address as usize).to_result()

        // Lock the mutex to prevent concurrent access
        let discrete_inputs_lock = self.discrete_inputs.lock().unwrap();
        // Return the value at the given address
        discrete_inputs_lock.get(address as usize)
            .cloned()
            .ok_or(ExceptionCode::IllegalDataAddress)
    }

    fn read_holding_register(&self, address: u16) -> Result<u16, ExceptionCode> {
        //self.holding_registers.get(address as usize).to_result()

        // Lock the mutex to prevent concurrent access
        let holding_registers_lock = self.holding_registers.lock().unwrap();
        // Return the value at the given address
        holding_registers_lock.get(address as usize)
            .cloned()
            .ok_or(ExceptionCode::IllegalDataAddress)
    }

    fn read_input_register(&self, address: u16) -> Result<u16, ExceptionCode> {
        //self.input_registers.get(address as usize).to_result()

        // Lock the mutex to prevent concurrent access
        let input_registers_lock = self.input_registers.lock().unwrap();
        // Return the value at the given address
        input_registers_lock.get(address as usize)
            .cloned()
            .ok_or(ExceptionCode::IllegalDataAddress)
    }

    fn write_single_coil(&mut self, value: Indexed<bool>) -> Result<(), ExceptionCode> {
        tracing::info!(
            "write single coil, index: {} value: {}",
            value.index,
            value.value
        );

        /*if let Some(coil) = self.coils.get_mut(value.index as usize) {
            *coil = value.value;
            Ok(())
        } else {
            Err(ExceptionCode::IllegalDataAddress)
        }*/

        // Lock the mutex for mutation
        let mut coils_lock = self.coils.lock().unwrap();
        // Update the value at the given address
        if let Some(coil) = coils_lock.get_mut(value.index as usize) {
            *coil = value.value;
            Ok(())
        } else {
            Err(ExceptionCode::IllegalDataAddress)
        }
    }

    fn write_single_register(&mut self, value: Indexed<u16>) -> Result<(), ExceptionCode> {
        tracing::info!(
            "write single register, index: {} value: {}",
            value.index,
            value.value
        );

        /*if let Some(reg) = self.holding_registers.get_mut(value.index as usize) {
            *reg = value.value;
            Ok(())
        } else {
            Err(ExceptionCode::IllegalDataAddress)
        }*/

        // Lock the mutex for mutation
        let mut holding_registers_lock = self.holding_registers.lock().unwrap();
        // Update the value at the given address
        if let Some(reg) = holding_registers_lock.get_mut(value.index as usize) {
            *reg = value.value;
            Ok(())
        } else {
            Err(ExceptionCode::IllegalDataAddress)
        }
    }

    fn write_multiple_coils(&mut self, values: WriteCoils) -> Result<(), ExceptionCode> {
        tracing::info!("write multiple coils {:?}", values.range);

        /*let mut result = Ok(());

        for value in values.iterator {
            if let Some(coil) = self.coils.get_mut(value.index as usize) {
                *coil = value.value;
            } else {
                result = Err(ExceptionCode::IllegalDataAddress)
            }
        }

        result*/

        // Lock the mutex for mutation
        let mut coils_lock = self.coils.lock().unwrap();
        // Update the values at the given addresses
        for value in values.iterator {
            if let Some(coil) = coils_lock.get_mut(value.index as usize) {
                *coil = value.value;
            } else {
                return Err(ExceptionCode::IllegalDataAddress);
            }
        }
        Ok(())
    }

    fn write_multiple_registers(&mut self, values: WriteRegisters) -> Result<(), ExceptionCode> {
        tracing::info!("write multiple registers {:?}", values.range);

        /*let mut result = Ok(());

        for value in values.iterator {
            if let Some(reg) = self.holding_registers.get_mut(value.index as usize) {
                *reg = value.value;
            } else {
                result = Err(ExceptionCode::IllegalDataAddress)
            }
        }

        result*/

        // Lock the mutex for mutation
        let mut holding_registers_lock = self.holding_registers.lock().unwrap();
        // Update the values at the given addresses
        for value in values.iterator {
            if let Some(reg) = holding_registers_lock.get_mut(value.index as usize) {
                *reg = value.value;
            } else {
                return Err(ExceptionCode::IllegalDataAddress);
            }
        }
        Ok(())
    }
}
// ANCHOR_END: request_handler

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    let args: Vec<String> = std::env::args().collect();
    let transport: &str = match &args[..] {
        [_, x] => x,
        _ => {
            eprintln!("please specify a transport:");
            eprintln!("usage: outstation <transport> (tcp, rtu, tls-ca, tls-self-signed)");
            exit(-1);
        }
    };
    match transport {
        "tcp" => run_tcp().await,
        #[cfg(feature = "serial")]
        "rtu" => run_rtu().await,
        #[cfg(feature = "tls")]
        "tls-ca" => run_tls(get_ca_chain_config()?).await,
        #[cfg(feature = "tls")]
        "tls-self-signed" => run_tls(get_self_signed_config()?).await,
        _ => {
            eprintln!(
                "unknown transport '{transport}', options are (tcp, rtu, tls-ca, tls-self-signed)"
            );
            exit(-1);
        }
    }
}

async fn run_tcp() -> Result<(), Box<dyn std::error::Error>> {
    let (handler, map) = create_handler();

    // ANCHOR: tcp_server_create
    let server = rodbus::server::spawn_tcp_server_task(
        1,
        "127.0.0.1:502".parse()?,
        map,
        AddressFilter::Any,
        DecodeLevel::default(),
    )
    .await?;
    // ANCHOR_END: tcp_server_create

    run_server(server, handler).await
}

#[cfg(feature = "serial")]
async fn run_rtu() -> Result<(), Box<dyn std::error::Error>> {
    let (handler, map) = create_handler();

    // ANCHOR: rtu_server_create
    let server = rodbus::server::spawn_rtu_server_task(
        "/dev/ttySIM1",
        rodbus::SerialSettings::default(),
        default_retry_strategy(),
        map,
        DecodeLevel::new(
            AppDecodeLevel::DataValues,
            FrameDecodeLevel::Payload,
            PhysDecodeLevel::Data,
        ),
    )?;
    // ANCHOR_END: rtu_server_create

    run_server(server, handler).await
}

#[cfg(feature = "tls")]
async fn run_tls(tls_config: TlsServerConfig) -> Result<(), Box<dyn std::error::Error>> {
    let (handler, map) = create_handler();

    // ANCHOR: tls_server_create
    let server = rodbus::server::spawn_tls_server_task_with_authz(
        1,
        "127.0.0.1:802".parse()?,
        map,
        ReadOnlyAuthorizationHandler::create(),
        tls_config,
        AddressFilter::Any,
        DecodeLevel::default(),
    )
    .await?;
    // ANCHOR_END: tls_server_create

    run_server(server, handler).await
}

fn create_handler() -> (
    ServerHandlerType<SimpleHandler>,
    ServerHandlerMap<SimpleHandler>,
) {
    // ANCHOR: handler_map_create
    let handler =
        SimpleHandler::new(vec![false; 10], vec![false; 10], vec![0; 10], vec![0; 10]).wrap();

    // map unit ids to a handler for processing requests
    let map = ServerHandlerMap::single(UnitId::new(1), handler.clone());
    // ANCHOR_END: handler_map_create

    (handler, map)
}

#[cfg(feature = "tls")]
fn get_self_signed_config() -> Result<TlsServerConfig, Box<dyn std::error::Error>> {
    use std::path::Path;
    // ANCHOR: tls_self_signed_config
    let tls_config = TlsServerConfig::new(
        Path::new("./certs/self_signed/entity1_cert.pem"),
        Path::new("./certs/self_signed/entity2_cert.pem"),
        Path::new("./certs/self_signed/entity2_key.pem"),
        None, // no password
        MinTlsVersion::V1_2,
        CertificateMode::SelfSigned,
    )?;
    // ANCHOR_END: tls_self_signed_config

    Ok(tls_config)
}

#[cfg(feature = "tls")]
fn get_ca_chain_config() -> Result<TlsServerConfig, Box<dyn std::error::Error>> {
    use std::path::Path;
    // ANCHOR: tls_ca_chain_config
    let tls_config = TlsServerConfig::new(
        Path::new("./certs/ca_chain/ca_cert.pem"),
        Path::new("./certs/ca_chain/server_cert.pem"),
        Path::new("./certs/ca_chain/server_key.pem"),
        None, // no password
        MinTlsVersion::V1_2,
        CertificateMode::AuthorityBased,
    )?;
    // ANCHOR_END: tls_ca_chain_config

    Ok(tls_config)
}

async fn run_server(
    mut server: ServerHandle,
    handler: ServerHandlerType<SimpleHandler>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = FramedRead::new(tokio::io::stdin(), LinesCodec::new());
    loop {
        match reader.next().await.unwrap()?.as_str() {
            "x" => return Ok(()),
            "ed" => {
                // enable decoding
                server
                    .set_decode_level(DecodeLevel::new(
                        AppDecodeLevel::DataValues,
                        FrameDecodeLevel::Header,
                        PhysDecodeLevel::Length,
                    ))
                    .await?;
            }
            "dd" => {
                // disable decoding
                server.set_decode_level(DecodeLevel::nothing()).await?;
            }
            "uc" => {
                let handler_lock = handler.lock().unwrap();
                let mut coils = handler_lock.coils.lock().unwrap();
                for coil in coils.iter_mut() {
                    *coil = !*coil;
                }
            }
            "udi" => {
                let handler_lock = handler.lock().unwrap();
                let mut discrete_inputs = handler_lock.discrete_inputs.lock().unwrap();
                for discrete_input in discrete_inputs.iter_mut() {
                    *discrete_input = !*discrete_input;
                }
            }
            "uhr" => {
                let handler_lock = handler.lock().unwrap();
                let mut holding_registers = handler_lock.holding_registers.lock().unwrap();
                for holding_register in holding_registers.iter_mut() {
                    *holding_register += 1;
                }
            }
            "uir" => {
                let handler_lock = handler.lock().unwrap();
                let mut input_registers = handler_lock.input_registers.lock().unwrap();
                for input_register in input_registers.iter_mut() {
                    *input_register += 1;
                }
            }
            _ => println!("unknown command"),
        }
    }
}

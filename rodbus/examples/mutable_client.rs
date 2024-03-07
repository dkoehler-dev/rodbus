use std::error::Error;
use std::net::{IpAddr, Ipv4Addr};
use std::process::exit;
use std::time::Duration;

use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};

use rodbus::client::*;
use rodbus::*;

// ANCHOR: runtime_init
#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    // ANCHOR_END: runtime_init

    // ANCHOR: logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();
    // ANCHOR_END: logging

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

struct LoggingListener;
impl<T> Listener<T> for LoggingListener
where
    T: std::fmt::Debug,
{
    fn update(&mut self, value: T) -> MaybeAsync<()> {
        tracing::info!("Channel Listener: {:?}", value);
        MaybeAsync::ready(())
    }
}

async fn run_tcp() -> Result<(), Box<dyn std::error::Error>> {
    // ANCHOR: create_tcp_channel
    let channel = spawn_tcp_client_task(
        HostAddr::ip(IpAddr::V4(Ipv4Addr::LOCALHOST), 10502),
        1,
        default_retry_strategy(),
        DecodeLevel::default(),
        Some(Box::new(LoggingListener)),
    );
    // ANCHOR_END: create_tcp_channel

    run_channel(channel).await
}

#[cfg(feature = "serial")]
async fn run_rtu() -> Result<(), Box<dyn std::error::Error>> {
    // ANCHOR: create_rtu_channel
    let channel = spawn_rtu_client_task(
        "/dev/ttySIM0",                    // path
        rodbus::SerialSettings::default(), // serial settings
        1,                                 // max queued requests
        default_retry_strategy(),          // retry delays
        DecodeLevel::new(
            AppDecodeLevel::DataValues,
            FrameDecodeLevel::Payload,
            PhysDecodeLevel::Nothing,
        ),
        Some(Box::new(LoggingListener)),
    );
    // ANCHOR_END: create_rtu_channel

    run_channel(channel).await
}

#[cfg(feature = "tls")]
async fn run_tls(tls_config: TlsClientConfig) -> Result<(), Box<dyn std::error::Error>> {
    // ANCHOR: create_tls_channel
    let channel = spawn_tls_client_task(
        HostAddr::ip(IpAddr::V4(Ipv4Addr::LOCALHOST), 10802),
        1,
        default_retry_strategy(),
        tls_config,
        DecodeLevel::new(
            AppDecodeLevel::DataValues,
            FrameDecodeLevel::Nothing,
            PhysDecodeLevel::Nothing,
        ),
        Some(Box::new(LoggingListener)),
    );
    // ANCHOR_END: create_tls_channel

    run_channel(channel).await
}

#[cfg(feature = "tls")]
fn get_self_signed_config() -> Result<TlsClientConfig, Box<dyn std::error::Error>> {
    use std::path::Path;
    // ANCHOR: tls_self_signed_config
    let tls_config = TlsClientConfig::self_signed(
        Path::new("./certs/self_signed/entity2_cert.pem"),
        Path::new("./certs/self_signed/entity1_cert.pem"),
        Path::new("./certs/self_signed/entity1_key.pem"),
        None, // no password
        MinTlsVersion::V1_2,
    )?;
    // ANCHOR_END: tls_self_signed_config

    Ok(tls_config)
}

#[cfg(feature = "tls")]
fn get_ca_chain_config() -> Result<TlsClientConfig, Box<dyn std::error::Error>> {
    use std::path::Path;
    // ANCHOR: tls_ca_chain_config
    let tls_config = TlsClientConfig::full_pki(
        Some("test.com".to_string()),
        Path::new("./certs/ca_chain/ca_cert.pem"),
        Path::new("./certs/ca_chain/client_cert.pem"),
        Path::new("./certs/ca_chain/client_key.pem"),
        None, // no password
        MinTlsVersion::V1_2,
    )?;
    // ANCHOR_END: tls_ca_chain_config

    Ok(tls_config)
}

fn print_read_result<T>(result: Result<Vec<Indexed<T>>, RequestError>)
where
    T: std::fmt::Display,
{
    match result {
        Ok(coils) => {
            for bit in coils {
                println!("index: {} value: {}", bit.index, bit.value);
            }
        }
        Err(rodbus::RequestError::Exception(exception)) => {
            println!("Modbus exception: {exception}");
        }
        Err(err) => println!("read error: {err}"),
    }
}

fn print_write_result<T>(result: Result<T, RequestError>) {
    match result {
        Ok(_) => {
            println!("write successful");
        }
        Err(rodbus::RequestError::Exception(exception)) => {
            println!("Modbus exception: {exception}");
        }
        Err(err) => println!("writer error: {err}"),
    }
}

async fn run_channel(mut channel: Channel) -> Result<(), Box<dyn std::error::Error>> {
    channel.enable().await?;

    // ANCHOR: request_param
    let params = RequestParam::new(UnitId::new(1), Duration::from_secs(1));
    // ANCHOR_END: request_param

    let mut reader = FramedRead::new(tokio::io::stdin(), LinesCodec::new());
    while let Some(line) = reader.next().await {
        let line = line?; // This handles the Some(Err(e)) case by returning Err(e)
        let parts = line.split_whitespace().collect::<Vec<&str>>();
        match parts.as_slice() {
            ["x"] => return Ok(()),
            ["ec"] => {
                channel.enable().await?;
            }
            ["dc"] => {
                channel.disable().await?;
            }
            ["ed"] => {
                channel
                    .set_decode_level(DecodeLevel::new(
                        AppDecodeLevel::DataValues,
                        FrameDecodeLevel::Payload,
                        PhysDecodeLevel::Data,
                    ))
                    .await?;
            }
            ["dd"] => {
                channel.set_decode_level(DecodeLevel::nothing()).await?;
            }
            ["scfc", fc_str, values @ ..] => {
                let fc = u8::from_str_radix(fc_str.trim_start_matches("0x"), 16).unwrap();
                let values: Vec<u16> = values.iter().filter_map(|&v| u16::from_str_radix(v.trim_start_matches("0x"), 16).ok()).collect();

                if (fc >= 65 && fc <= 72) || (fc >= 100 && fc <= 110) {
                    if values.len() >= 2 {
                        let byte_count_in = values[0] as u8;
                        let byte_count_out = values[1] as u8;

                        let result = channel
                            .send_custom_function_code(
                                params,
                                CustomFunctionCode::new(fc, byte_count_in, byte_count_out, values[2..].to_vec())
                            )
                            .await;
                        print_write_result(result);
                    } else {
                        println!("Error: missing arguments.");
                    }
                } else {
                    match fc {
                        0x01 => {
                            // ANCHOR: read_coils
                            let start = values[0];
                            let count = values[1];
                            let result = channel
                                .read_coils(params, AddressRange::try_from(start, count).unwrap())
                                .await;
                            // ANCHOR_END: read_coils
                            print_read_result(result);
                        }
                        0x02 => {
                            // ANCHOR: read_discrete_inputs
                            let start = values[0];
                            let count = values[1];
                            let result = channel
                                .read_discrete_inputs(params, AddressRange::try_from(start, count).unwrap())
                                .await;
                            // ANCHOR_END: read_discrete_inputs
                            print_read_result(result);
                        }
                        0x03 => {
                            // ANCHOR: read_holding_registers
                            let start = values[0];
                            let count = values[1];
                            let result = channel
                                .read_holding_registers(params, AddressRange::try_from(start, count).unwrap())
                                .await;
                            // ANCHOR_END: read_holding_registers
                            print_read_result(result);
                        }
                        0x04 => {
                            // ANCHOR: read_input_registers
                            let start = values[0];
                            let count = values[1];
                            let result = channel
                                .read_input_registers(params, AddressRange::try_from(start, count).unwrap())
                                .await;
                            // ANCHOR_END: read_input_registers
                            print_read_result(result);
                        }
                        0x05 => {
                            // ANCHOR: write_single_coil
                            let address = values[0];
                            let value = values[1] != 0;
                            let result = channel
                                .write_single_coil(params, Indexed::new(address, value))
                                .await;
                            // ANCHOR_END: write_single_coil
                            print_write_result(result);
                        }
                        0x06 => {
                            // ANCHOR: write_single_register
                            let address = values[0];
                            let value = values[1];
                            let result = channel
                                .write_single_register(params, Indexed::new(address, value))
                                .await;
                            // ANCHOR_END: write_single_register
                            print_write_result(result);
                        }
                        0x0F => {
                            // ANCHOR: write_multiple_coils
                            let start = values[0];
                            // The subsequent values are the coil states, convert the values to booleans
                            let coils: Vec<bool> = values[1..].iter().map(|&v| v != 0).collect();
                            let result = channel
                                .write_multiple_coils(params, WriteMultiple::from(start, coils).unwrap())
                                .await;
                            // ANCHOR_END: write_multiple_coils
                            print_write_result(result);
                        }
                        0x10 => {
                            // ANCHOR: write_multiple_registers
                            let start = values[0];
                            let registers: Vec<u16> = values[1..].to_vec();
                            let result = channel
                                .write_multiple_registers(params, WriteMultiple::from(start, registers).unwrap())
                                .await;
                            // ANCHOR_END: write_multiple_registers
                            print_write_result(result);
                        }
                        _ => println!("unknown function code"),
                    };
                    println!("Error: CFC number is not inside the range of 65-72 or 100-110.");
                }
            }
            _ => println!("unknown command"),
        }
    }
    Ok(())
}

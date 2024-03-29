use std::error::Error;
use std::net::{IpAddr, Ipv4Addr};
use std::process::exit;
use std::time::Duration;
use std::vec;

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
        HostAddr::ip(IpAddr::V4(Ipv4Addr::LOCALHOST), 11502),
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
        HostAddr::ip(IpAddr::V4(Ipv4Addr::LOCALHOST), 11802),
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
        Ok(registers) => {
            for register in registers {
                println!("index: {} value: {}", register.index, register.value);
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
    let params = RequestParam::new(UnitId::new(1), Duration::from_secs(900));
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
            ["rc"] => {
                // ANCHOR: read_coils
                let result = channel
                    .read_coils(params, AddressRange::try_from(0, 5).unwrap())
                    .await;
                // ANCHOR_END: read_coils
                print_read_result(result);
            }
            ["smfc", fc_str, values @ ..] => {
                let fc = u8::from_str_radix(fc_str.trim_start_matches("0x"), 16).unwrap();
                
                // If values were supplied, use them. Otherwise use example request data
                if !values.is_empty() {
                    let values = values.iter().filter_map(|&v| u8::from_str_radix(v.trim_start_matches("0x"), 16).ok()).collect();
                    let result = channel
                        .send_mutable_function_code(
                            params,
                            MutableFunctionCode::new(fc, values),
                        )
                        .await;
                    print_write_result(result);
                } else {
                    let data = match fc {
                        // Undefined FCs
                        0 | 9 | 10 | 13 | 14 | 18 | 19 | 25 | 26 | 27 | 28 | 29 | 30 | 31 | 32 | 33 | 34 | 35 | 36 | 37 | 38 | 39 | 40 | 41 | 42 | 44 | 45 | 46 | 47 | 48 | 49 | 50 | 
                        51 | 52 | 53 | 54 | 55 | 56 | 57 | 58 | 59 | 60 | 61 | 62 | 63 | 64 | 73 | 74 | 75 | 76 | 77 | 78 | 79 | 80 | 81 | 82 | 83 | 84 | 85 | 86 | 87 | 88 | 89 | 90 | 91 | 
                        92 | 93 | 94 | 95 | 96 | 97 | 98 | 99 | 111 | 112 | 113 | 114 | 115 | 116 | 117 | 118 | 119 | 120 | 121 | 122 | 123 | 124 | 125 | 126 | 127 | 128 | 129 | 130 | 131 | 
                        132 | 133 | 134 | 135 | 136 | 137 | 138 | 139 | 140 | 141 | 142 | 143 | 144 | 145 | 146 | 147 | 148 | 149 | 150 | 151 | 152 | 153 | 154 | 155 | 156 | 157 | 158 | 159 | 
                        160 | 161 | 162 | 163 | 164 | 165 | 166 | 167 | 168 | 169 | 170 | 171 | 172 | 173 | 174 | 175 | 176 | 177 | 178 | 179 | 180 | 181 | 182 | 183 | 184 | 185 | 186 | 187 | 
                        188 | 189 | 190 | 191 | 192 | 193 | 194 | 195 | 196 | 197 | 198 | 199 | 200 | 201 | 202 | 203 | 204 | 205 | 206 | 207 | 208 | 209 | 210 | 211 | 212 | 213 | 214 | 215 | 
                        216 | 217 | 218 | 219 | 220 | 221 | 222 | 223 | 224 | 225 | 226 | 227 | 228 | 229 | 230 | 231 | 232 | 233 | 234 | 235 | 236 | 237 | 238 | 239 | 240 | 241 | 242 | 243 | 
                        244 | 245 | 246 | 247 | 248 | 249 | 250 | 251 | 252 | 253 | 254 | 255 => vec![],
                        // Read Coils - start: 0, quantity: 5
                        1 => vec![0, 0, 0, 5],
                        // Read Discrete Inputs - start: 0, quantity: 5
                        2 => vec![0, 0, 0, 5],
                        // Read Holding Registers - start: 0, quantity: 5
                        3 => vec![0, 0, 0, 5],
                        // Read Input Registers - start: 0, quantity: 5
                        4 => vec![0, 0, 0, 5],
                        // Write Single Coil - address: 0, value: ON (FF 00)
                        5 => vec![0, 0, 255, 0],
                        // Write Single Register - address: 0, value: 1
                        6 => vec![0, 0, 0, 1],
                        // Read Exception Status - Serial Line only - Not implemented (IllegalFunction)
                        7 => vec![],
                        // Diagnostic - sub function: 0, data: [0xA5, 0x37]  - Serial Line only - Not implemented (IllegalFunction)
                        8 => vec![0, 0, 165, 55],
                        // Get Comm Event Counter - Serial Line only - Not implemented (IllegalFunction)
                        11 => vec![],
                        // Get Comm Event Log - Serial Line only - Not implemented (IllegalFunction)
                        12 => vec![],
                        // Write Multiple Coils - start: 0, quantity: 8, byte count: 1, values: 1111 1111
                        15 => vec![0, 0, 0, 8, 1, 255],
                        // Write Multiple Registers - start: 0, quantity: 5, byte count: 10, values: 1, 2, 3, 4, 5
                        16 => vec![0, 0, 0, 5, 10, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5],
                        // Report Slave ID  - Serial Line only - Not implemented (IllegalFunction)
                        17 => vec![],
                        // Read File Record - Not implemented (IllegalFunction)
                        20 => vec![],
                        // Write File Record - Not implemented (IllegalFunction)
                        21 => vec![],
                        // Mask Write Register - Not implemented (IllegalFunction)
                        22 => vec![],
                        // Read/Write Multiple Registers - Not implemented (IllegalFunction)
                        23 => vec![],
                        // Read FIFO Queue - Not implemented (IllegalFunction)
                        24 => vec![],
                        // Read Device Identification - Not implemented (IllegalFunction)
                        43 => vec![],
                        // Custom FCs
                        65 | 66 | 67 | 68 | 69 | 70 | 71 | 72 | 100 | 101 | 102 | 103 | 104 | 105 | 106 | 107 | 108 | 109 | 110 => vec![fc, 0xC0, 0xDE],
                    };
                    let result = channel
                        .send_mutable_function_code(
                            params,
                            MutableFunctionCode::new(fc, data),
                        )
                        .await;
                    print_write_result(result);
                }
            }
            _ => println!("unknown command"),
        }
    }
    Ok(())
}
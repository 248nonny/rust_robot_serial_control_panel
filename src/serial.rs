use crate::serial_protocol;
use serialport::{available_ports, SerialPortType};
use std::io::{self, Write};

use crate::serial_protocol::MessageCode;

#[derive(Debug)]
pub enum MsgElem {
    Code(MessageCode),
    F32(f32),
    U32(u32),
    I32(i32),
}

impl MsgElem {
    fn to_u8_vec(&self) -> Vec<u8> {
        let mut output = Vec::with_capacity(4);
        match self {
            Self::Code(x) => output.push(*x as u32 as u8),
            Self::F32(x) => output.extend(x.to_le_bytes()),
            Self::U32(x) => output.extend(x.to_le_bytes()),
            Self::I32(x) => output.extend(x.to_le_bytes()),
        }

        output
    }
}

pub fn list_ports() {
    match available_ports() {
        Ok(mut ports) => {
            ports.sort_by_key(|i| i.port_name.clone());

            match ports.len() {
                0 => println!("No ports found!"),
                1 => println!("1 port found:"),
                n => println!("{} ports found:", n),
            };

            for p in ports {
                println!("  {}", p.port_name);
                match p.port_type {
                    SerialPortType::UsbPort(info) => {
                        println!("       type: USB");
                        println!("        VID: {:04x}", info.vid);
                        println!("        PID: {:04x}", info.pid);
                        println!(
                            "         SN: {}",
                            info.serial_number.as_ref().map_or("", String::as_str),
                        );
                        println!(
                            "     Mnfctr: {}",
                            info.manufacturer.as_ref().map_or("", String::as_str),
                        );
                        println!(
                            "    product: {}",
                            info.product.as_ref().map_or("", String::as_str),
                        );
                    }
                    SerialPortType::BluetoothPort => {
                        println!("      type: Bluetooth");
                    }
                    SerialPortType::PciPort => {
                        println!("      type: PCI");
                    }
                    SerialPortType::Unknown => {
                        println!("      type: Unknown");
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("{:?}", e);
            eprintln!("Error listing Serial ports...");
        }
    }
}

fn convert_message(message: &[MsgElem]) -> Vec<u8> {
    let mut converted_message: Vec<u8> = Vec::with_capacity(message.len() * 4 + 2);
    converted_message.push(serial_protocol::MessageCode::MSG_START as u32 as u8);
    for item in message {
        converted_message.extend(item.to_u8_vec());
    }
    converted_message.push(serial_protocol::MessageCode::MSG_END as u32 as u8);
    converted_message
}

pub fn send_message(port: &mut Box<dyn serialport::SerialPort + 'static>, message: &[MsgElem]) {
    let converted_message = convert_message(&message);

    // for debugging, print message and serial output:
    print!("[Rust] Sending message to esp32:");
    for elem in message {
        print!(" {:?}", elem);
    }
    print!("\n  Binary msg: ");
    io::stdout().write_all(&converted_message).unwrap();
    io::stdout().flush().unwrap();

    match port.write(&converted_message[..]) {
        Ok(_) => {}
        Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
        Err(e) => eprintln!("{:?}", e),
    }
}

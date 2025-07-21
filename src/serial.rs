use crate::serial_protocol;
use serialport::{available_ports, SerialPortType};
use std::io::{self, Write};

use crate::serial_protocol::MessageCode;

#[derive(PartialEq, Debug)]
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

fn u8_to_code(input: u8) -> Option<MessageCode> {
    match input {
        x if x == MessageCode::MSG_START as u32 as u8 => Some(MessageCode::MSG_START),
        x if x == MessageCode::FLOAT_AHEAD as u32 as u8 => Some(MessageCode::FLOAT_AHEAD),
        x if x == MessageCode::UINT_AHEAD as u32 as u8 => Some(MessageCode::UINT_AHEAD),
        x if x == MessageCode::INT_AHEAD as u32 as u8 => Some(MessageCode::INT_AHEAD),
        x if x == MessageCode::SET as u32 as u8 => Some(MessageCode::SET),
        x if x == MessageCode::GET as u32 as u8 => Some(MessageCode::GET),
        x if x == MessageCode::ARM as u32 as u8 => Some(MessageCode::ARM),
        x if x == MessageCode::SHOULDER as u32 as u8 => Some(MessageCode::SHOULDER),
        x if x == MessageCode::ELBOW as u32 as u8 => Some(MessageCode::ELBOW),
        x if x == MessageCode::CLAW as u32 as u8 => Some(MessageCode::CLAW),
        x if x == MessageCode::DRIVE_TRAIN as u32 as u8 => Some(MessageCode::DRIVE_TRAIN),
        x if x == MessageCode::LIDAR as u32 as u8 => Some(MessageCode::LIDAR),
        x if x == MessageCode::MAGNETOMETER as u32 as u8 => Some(MessageCode::MAGNETOMETER),
        x if x == MessageCode::IR_BEACON as u32 as u8 => Some(MessageCode::IR_BEACON),
        x if x == MessageCode::TAPE_SENSOR as u32 as u8 => Some(MessageCode::TAPE_SENSOR),
        x if x == MessageCode::ANGLE as u32 as u8 => Some(MessageCode::ANGLE),
        x if x == MessageCode::VELOCITY as u32 as u8 => Some(MessageCode::VELOCITY),
        x if x == MessageCode::PID_ERROR as u32 as u8 => Some(MessageCode::PID_ERROR),
        x if x == MessageCode::PID_SETPOINT as u32 as u8 => Some(MessageCode::PID_SETPOINT),
        x if x == MessageCode::PID_ACCUMULATOR as u32 as u8 => Some(MessageCode::PID_ACCUMULATOR),
        x if x == MessageCode::PID_KP as u32 as u8 => Some(MessageCode::PID_KP),
        x if x == MessageCode::PID_KI as u32 as u8 => Some(MessageCode::PID_KI),
        x if x == MessageCode::PID_KD as u32 as u8 => Some(MessageCode::PID_KD),
        x if x == MessageCode::PID_OUTPUT as u32 as u8 => Some(MessageCode::PID_OUTPUT),
        x if x == MessageCode::RAW as u32 as u8 => Some(MessageCode::RAW),
        x if x == MessageCode::CONVERTED as u32 as u8 => Some(MessageCode::CONVERTED),
        x if x == MessageCode::LEFT as u32 as u8 => Some(MessageCode::LEFT),
        x if x == MessageCode::RIGHT as u32 as u8 => Some(MessageCode::RIGHT),
        x if x == MessageCode::MSG_TYPE_COUNTER as u32 as u8 => Some(MessageCode::MSG_TYPE_COUNTER),
        x if x == MessageCode::MSG_END as u32 as u8 => Some(MessageCode::MSG_END),
        _ => None,
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

fn parse_to_message(buffer: &mut [u8], index: usize, message: &mut Vec<MsgElem>) -> usize {
    if buffer[index - 1] == MessageCode::MSG_END as u32 as u8 {
        let mut i: usize = 1;
        while i < index {
            println!("asdas");
            message.push(match u8_to_code(buffer[i]).unwrap() {
                MessageCode::FLOAT_AHEAD => {
                    let elem =
                        MsgElem::F32(f32::from_le_bytes(buffer[i + 1..i + 5].try_into().unwrap()));
                    i += 4;
                    elem
                }
                MessageCode::UINT_AHEAD => {
                    let elem =
                        MsgElem::U32(u32::from_le_bytes(buffer[i + 1..i + 5].try_into().unwrap()));
                    i += 4;
                    elem
                }
                MessageCode::INT_AHEAD => {
                    let elem =
                        MsgElem::I32(i32::from_le_bytes(buffer[i + 1..i + 5].try_into().unwrap()));
                    i += 4;
                    elem
                }
                x => MsgElem::Code(x),
            });
            i += 1;
        }
        message.pop();
        index
    } else {
        0
    }
}

pub fn read_message(
    port: &mut Box<dyn serialport::SerialPort + 'static>,
    message: &mut Vec<MsgElem>,
    buffer: &mut [u8; 1024],
    index: &mut usize,
) -> usize {
    // static mut buffer: [u8; 1024] = [0; 1024];
    // static mut index: usize = 0;

    if buffer[0] != MessageCode::MSG_START as u32 as u8 {
        *index = 0;
    }

    match port.read(&mut buffer[..]) {
        Ok(t) => {
            if *index + t > buffer.len() {
                *index = 0;
                return 0;
            }
            io::stdout().write_all(&buffer[*index..*index + t]).unwrap();
            io::stdout().flush().unwrap();
            println!();
            *index += t;
            let out = parse_to_message(buffer, *index, message);
            if out > 0 {
                *index = 0;
            }
            return out;
        }
        Err(ref e) if e.kind() == io::ErrorKind::TimedOut => 0,
        Err(e) => {
            eprintln!("{:?}", e);
            return 0;
        }
    }
}

use crate::serial_protocol;
use serialport::{available_ports, SerialPortType};
use std::{
    io::{self, Write},
    mem::discriminant,
};

use crate::serial_protocol::MessageCode;

#[derive(PartialEq, Debug, Clone)]
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

pub struct MessageBuffer {
    buf: Vec<u8>,
}

impl MessageBuffer {
    pub fn new() -> MessageBuffer {
        MessageBuffer { buf: Vec::new() }
    }

    pub fn read_serial(&mut self, port: &mut Box<dyn serialport::SerialPort + 'static>) {
        let mut buf: [u8; 1024] = [0; 1024];

        match port.read(&mut buf[..]) {
            Ok(t) => {
                let start_len = self.buf.len();
                self.buf.extend_from_slice(&buf[..t]);

                print!("[ESP]: ");
                io::stdout().write_all(&buf[..t]).unwrap();
                io::stdout().flush().unwrap();
                println!();
            }
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {}
            Err(e) => {
                eprintln!("{:?}", e);
                panic!();
            }
        }
    }

    pub fn parse_message(&mut self) -> Option<Vec<MsgElem>> {
        if self.buf.len() >= 2000 {
            self.buf.clear();
        }

        let start_index = self
            .buf
            .iter()
            .position(|x| *x == MessageCode::MSG_START as u32 as u8)?;

        let end_index = self
            .buf
            .iter()
            .skip(start_index)
            .position(|x| *x == MessageCode::MSG_END as u32 as u8)?
            + start_index;

        let message = parse_to_message(&self.buf[start_index + 1..end_index]);
        self.buf = self.buf[end_index + 1..].to_vec();

        message
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
        x if x == MessageCode::PID as u32 as u8 => Some(MessageCode::PID),
        x if x == MessageCode::ARM as u32 as u8 => Some(MessageCode::ARM),
        x if x == MessageCode::TTBL as u32 as u8 => Some(MessageCode::TTBL),
        x if x == MessageCode::SHOULDER as u32 as u8 => Some(MessageCode::SHOULDER),
        x if x == MessageCode::ELBOW as u32 as u8 => Some(MessageCode::ELBOW),
        x if x == MessageCode::CLAW as u32 as u8 => Some(MessageCode::CLAW),
        x if x == MessageCode::ENCODER_MOTOR as u32 as u8 => Some(MessageCode::ENCODER_MOTOR),
        x if x == MessageCode::DRIVE_BASE as u32 as u8 => Some(MessageCode::DRIVE_BASE),
        x if x == MessageCode::LIDAR as u32 as u8 => Some(MessageCode::LIDAR),
        x if x == MessageCode::MAGNETOMETER as u32 as u8 => Some(MessageCode::MAGNETOMETER),
        x if x == MessageCode::IR_BEACON as u32 as u8 => Some(MessageCode::IR_BEACON),
        x if x == MessageCode::TAPE_SENSOR as u32 as u8 => Some(MessageCode::TAPE_SENSOR),
        x if x == MessageCode::ODOMETRY as u32 as u8 => Some(MessageCode::ODOMETRY),
        x if x == MessageCode::ANGLE as u32 as u8 => Some(MessageCode::ANGLE),
        x if x == MessageCode::VELOCITY as u32 as u8 => Some(MessageCode::VELOCITY),
        x if x == MessageCode::PID_ERROR as u32 as u8 => Some(MessageCode::PID_ERROR),
        x if x == MessageCode::PID_SETPOINT as u32 as u8 => Some(MessageCode::PID_SETPOINT),
        x if x == MessageCode::PID_ACCUMULATOR as u32 as u8 => Some(MessageCode::PID_ACCUMULATOR),
        x if x == MessageCode::PID_KP as u32 as u8 => Some(MessageCode::PID_KP),
        x if x == MessageCode::PID_KI as u32 as u8 => Some(MessageCode::PID_KI),
        x if x == MessageCode::PID_KD as u32 as u8 => Some(MessageCode::PID_KD),
        x if x == MessageCode::PID_OUTPUT as u32 as u8 => Some(MessageCode::PID_OUTPUT),
        x if x == MessageCode::ALL as u32 as u8 => Some(MessageCode::ALL),
        x if x == MessageCode::NONE as u32 as u8 => Some(MessageCode::NONE),
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
    // print!("[Rust] Sending message to esp32:");
    // for elem in message {
    // print!(" {:?}", elem);
    // }
    // print!("\n  Binary msg: ");
    // io::stdout().write_all(&converted_message).unwrap();
    // io::stdout().flush().unwrap();

    match port.write(&converted_message[..]) {
        Ok(_) => {
            // println!("Sent message successfully.")
        }
        Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
        Err(e) => eprintln!("{:?}", e),
    }
}

fn parse_to_message(buffer: &[u8]) -> Option<Vec<MsgElem>> {
    let mut message: Vec<MsgElem> = Vec::with_capacity(128);
    let mut i: usize = 0;
    while i < buffer.len() {
        message.push(match u8_to_code(buffer[i]).unwrap_or(MessageCode::NONE) {
            MessageCode::FLOAT_AHEAD => {
                let elem;
                if i + 5 <= buffer.len() {
                    elem =
                        MsgElem::F32(f32::from_le_bytes(buffer[i + 1..i + 5].try_into().unwrap()));
                } else {
                    elem = MsgElem::Code(MessageCode::NONE);
                }
                i += 4;
                elem
            }
            MessageCode::UINT_AHEAD => {
                let elem;
                if i + 5 <= buffer.len() {
                    elem =
                        MsgElem::U32(u32::from_le_bytes(buffer[i + 1..i + 5].try_into().unwrap()));
                } else {
                    elem = MsgElem::Code(MessageCode::NONE);
                }
                i += 4;
                elem
            }
            MessageCode::INT_AHEAD => {
                let elem;
                if i + 5 <= buffer.len() {
                    elem =
                        MsgElem::I32(i32::from_le_bytes(buffer[i + 1..i + 5].try_into().unwrap()));
                } else {
                    elem = MsgElem::Code(MessageCode::NONE);
                }
                i += 4;
                elem
            }
            x => MsgElem::Code(x),
        });
        i += 1;
    }
    Some(message)
}

pub fn compare_messages(msg1: &[MsgElem], msg2: &[MsgElem]) -> bool {
    if msg1.len() != msg2.len() {
        return false;
    }

    msg1.iter()
        .zip(msg2.iter())
        .all(|(x, y)| discriminant(x) == discriminant(y))
}

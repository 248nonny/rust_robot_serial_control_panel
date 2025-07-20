use serialport::{available_ports, SerialPortType};

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

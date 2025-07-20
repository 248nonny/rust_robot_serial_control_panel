mod serial;

use serialport::{DataBits, StopBits};
use std::{
    io::{self, Write},
    time::{Duration, Instant},
};

fn main() {
    serial::list_ports();

    let string = "AMOGuS!!!!\n";

    let port_name = "/dev/ttyUSB0";
    let baud: u32 = 115200;
    let stop_bits = StopBits::One;
    let data_bits = DataBits::Eight;

    let port = serialport::new(port_name, baud)
        .stop_bits(stop_bits)
        .data_bits(data_bits)
        .timeout(Duration::from_millis(10))
        .open();

    match port {
        Ok(mut port) => {
            let mut serial_buf: Vec<u8> = vec![0; 1000];
            println!("Receiving data on {} at {} baud:", &port_name, &baud);

            let mut t = Instant::now();

            loop {
                if t.elapsed() > Duration::from_millis(1000) {
                    match port.write(string.as_bytes()) {
                        Ok(_) => {}
                        Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
                        Err(e) => eprintln!("{:?}", e),
                    }
                    t = Instant::now();
                }
                match port.read(serial_buf.as_mut_slice()) {
                    Ok(t) => {
                        // io::stdout().write_all(&serial_buf[..t]).unwrap();
                        // io::stdout().flush().unwrap();
                        let f = f32::from_le_bytes(serial_buf[0..4].try_into().unwrap());
                        println!("output: {}", f);
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
                    Err(e) => eprintln!("{:?}", e),
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to open {}. Err: {}", port_name, e);
        }
    }
}

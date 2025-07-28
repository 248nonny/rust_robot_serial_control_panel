mod serial;
mod serial_protocol;

use serialport::{available_ports, DataBits, SerialPortInfo, StopBits};
use std::{
    io::{self, Write},
    time::{Duration, Instant},
};

use serial::MsgElem;
use serial::MsgElem::*;
use serial_protocol::MessageCode::*;

use eframe::{egui, glow::CONTEXT_FLAG_ROBUST_ACCESS_BIT};
use egui_plot::{Line, Plot, PlotPoints};

fn main() -> Result<(), eframe::Error> {
    // serial::list_ports();

    let string = "AMOGuS!!!!\n";

    // let port_name = "/dev/ttyUSB0";

    // let port = serialport::new(port_name, baud)
    //     .stop_bits(stop_bits)
    //     .data_bits(data_bits)
    //     .timeout(Duration::from_millis(10))
    //     .open();

    // let test_msg: [MsgElem; 5] = [
    //     Code(GET),
    //     Code(ARM),
    //     Code(ELBOW),
    //     Code(ANGLE),
    //     I32(0x2d2d2d2d),
    // ];

    // match port {
    //     Ok(mut port) => {
    //         let mut serial_buf: Vec<u8> = vec![0; 1000];
    //         println!("Receiving data on {} at {} baud:", &port_name, &baud);

    //         let mut t = Instant::now();

    //         loop {
    //             if t.elapsed() > Duration::from_millis(1000) {
    //                 serial::send_message(&mut port, &test_msg);
    //                 // match port.write(string.as_bytes()) {
    //                 //     Ok(_) => {}
    //                 //     Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
    //                 //     Err(e) => eprintln!("{:?}", e),
    //                 // }

    //                 // match port.write(&test_msg) {
    //                 //     Ok(_) => {}
    //                 //     Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
    //                 //     Err(e) => eprintln!("{:?}", e),
    //                 // }

    //                 t = Instant::now();
    //             }
    //             match port.read(serial_buf.as_mut_slice()) {
    //                 Ok(t) => {
    //                     io::stdout().write_all(&serial_buf[..t]).unwrap();
    //                     io::stdout().flush().unwrap();
    //                     println!();
    //                     // let f = f32::from_le_bytes(serial_buf[0..4].try_into().unwrap());
    //                     // println!("output: {}", f);
    //                 }
    //                 Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
    //                 Err(e) => eprintln!("{:?}", e),
    //             }
    //         }
    //     }
    //     Err(e) => {
    //         eprintln!("Failed to open {}. Err: {}", port_name, e);
    //         Err(e)
    //     }
    // }

    let options = eframe::NativeOptions::default();

    eframe::run_native(
        "Robot Serial Interface",
        options,
        Box::new(|_cc| Ok(Box::new(SerialInterfaceApp::new()))),
    )
}

#[derive(PartialEq)]
enum View {
    PIDTuning,
    ArmControl,
    LidarTuning,
}

struct SerialInterfaceApp {
    pid_histogram: Vec<[f32; 6]>,
    view: View,

    available_ports: Vec<SerialPortInfo>,
    port_name: String,
    port: Option<Box<dyn serialport::SerialPort + 'static>>,
}

impl SerialInterfaceApp {
    fn new() -> Self {
        let available_ports = match available_ports() {
            Ok(x) => x,
            Err(_) => Vec::new(),
        };

        Self {
            pid_histogram: Vec::new(),
            view: View::PIDTuning,
            available_ports,
            port_name: String::new(),
            port: None,
        }
    }
}

impl eframe::App for SerialInterfaceApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::SidePanel::left("Serial Connection")
            .resizable(false)
            .show(ctx, |ui| {
                if ui.button("Refresh Serial Connections").clicked() {
                    match available_ports() {
                        Ok(x) => self.available_ports = x,
                        Err(_) => {}
                    }

                    // println!("{:?}", self.available_ports);
                };

                egui::ComboBox::from_label("Available Ports")
                    .selected_text(format!("{:?}", self.port_name))
                    .show_ui(ui, |ui| {
                        for v in &self.available_ports[..] {
                            let val = v.port_name.clone();
                            ui.selectable_value(&mut self.port_name, val.clone(), val);
                        }
                    });

                if ui.button("Connect").clicked() {
                    let baud: u32 = 115200;
                    let stop_bits = StopBits::One;
                    let data_bits = DataBits::Eight;
                    let timeout = Duration::from_millis(2);

                    println!("Connecting to {}.", self.port_name);
                    self.port = match (serialport::new(&self.port_name, baud)
                        .stop_bits(stop_bits)
                        .data_bits(data_bits)
                        .timeout(timeout)
                        .open())
                    {
                        Ok(x) => Some(x),
                        Err(_) => None,
                    };

                    println!("Connected to {:?}", self.port);
                }

                ui.label(format!(
                    "Connected to {}",
                    match &self.port {
                        Some(x) => x
                            .name()
                            .unwrap_or(String::from_utf8("None".into()).unwrap()),
                        None => String::from_utf8("None".into()).unwrap(),
                    },
                ));
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal_top(|ui| {
                ui.radio_value(&mut self.view, View::PIDTuning, "PID");
                ui.radio_value(&mut self.view, View::ArmControl, "Arm Control");
                ui.radio_value(&mut self.view, View::LidarTuning, "Lidar Tuning");
            });

            ui.label("Hello, world!");
            if ui.button("Click me!").clicked() {
                println!("button clicked!");
            }
        });
    }
}

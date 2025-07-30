mod ring_buffer;

mod serial;
mod serial_protocol;

use serialport::{available_ports, DataBits, SerialPortInfo, StopBits};
use std::{
    collections::btree_map::Values,
    io::{self, Write},
    time::{Duration, Instant},
};

use serial::MsgElem::*;
use serial::{compare_messages, read_message, send_message, MsgElem};
use serial_protocol::MessageCode::{self, *};

use ring_buffer::RingBuffer;

use eframe::{
    egui::{self, Color32},
    glow::CONTEXT_FLAG_ROBUST_ACCESS_BIT,
};
use egui_plot::{Line, Plot, PlotPoints};

fn main() -> Result<(), eframe::Error> {
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
    OdoTracking,
    ArmControl,
    LidarTuning,
}

#[derive(PartialEq, Clone)]
enum PIDTarget {
    EncoderMotor,
    DriveBase,
    Shoulder,
}

fn pid_target_to_msg(target: PIDTarget) -> MessageCode {
    match target {
        PIDTarget::EncoderMotor => MessageCode::ENCODER_MOTOR,
        PIDTarget::DriveBase => MessageCode::DRIVE_BASE,
        PIDTarget::Shoulder => MessageCode::SHOULDER,
    }
}

#[derive(Debug)]
struct Pos {
    x: f32,
    y: f32,
    theta: f32,
}

struct SerialInterfaceApp {
    // Store 6 values:
    //   error
    //   setpoint
    //   p_output
    //   i_output
    //   d_output
    pid_histogram: RingBuffer<[f64; 5]>,
    position_histogram: Vec<Pos>,
    position_plot_angle: f32,
    position_histogram_front: Vec<Pos>,
    view: View,

    available_ports: Vec<SerialPortInfo>,
    port_name: String,
    port: Option<Box<dyn serialport::SerialPort + 'static>>,

    arm_r: f32,
    arm_h: f32,
    ttbl_sensitivity: f32,

    serial_buffer: [u8; 1024],
    serial_buffer_index: usize,

    pid_target: PIDTarget,

    kp: f32,
    ki: f32,
    kd: f32,
    max_ce: f32,

    base_speed: f32,
    tape_following: bool,
}

impl SerialInterfaceApp {
    fn new() -> Self {
        let available_ports = match available_ports() {
            Ok(x) => x,
            Err(_) => Vec::new(),
        };

        Self {
            pid_histogram: RingBuffer::new(128),
            position_histogram: Vec::new(),
            position_plot_angle: 0.0,
            position_histogram_front: Vec::new(),
            view: View::PIDTuning,
            available_ports,
            port_name: String::new(),
            port: None,

            arm_r: 10.0,
            arm_h: 10.0,
            ttbl_sensitivity: 1.0,

            serial_buffer: [0; 1024],
            serial_buffer_index: 0,

            pid_target: PIDTarget::Shoulder,

            kp: 0.0,
            ki: 0.0,
            kd: 0.0,
            max_ce: 0.0,

            base_speed: 0.0,
            tape_following: false,
        }
    }
}

const PID_SINGLE_UPDATE_MESSAGE: [MsgElem; 6] =
    [Code(PID), F32(0.0), F32(0.0), F32(0.0), F32(0.0), F32(0.0)];

const ODOMETRY_POS_MESSAGE: [MsgElem; 4] = [Code(ODOMETRY), F32(0.0), F32(0.0), F32(0.0)];

impl eframe::App for SerialInterfaceApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Update as fast as possible lool.
        ctx.request_repaint();

        // println!("ASDADS");

        let message;

        if let Some(port) = self.port.as_mut() {
            message = read_message(port, &mut self.serial_buffer, &mut self.serial_buffer_index);

            if let Some(message) = message {
                if compare_messages(&message, &PID_SINGLE_UPDATE_MESSAGE) {
                    let new_elem = message
                        .iter()
                        .filter_map(|i| {
                            if let F32(x) = i {
                                Some(*x as f64)
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .try_into();
                    match new_elem {
                        Ok(x) => self.pid_histogram.push(x),
                        Err(_) => {
                            println!("Error parsing PID message...");
                        }
                    }
                } else if compare_messages(&message, &ODOMETRY_POS_MESSAGE) {
                    let v = message
                        .iter()
                        .filter_map(|i| if let F32(x) = i { Some(*x) } else { None })
                        .collect::<Vec<_>>();
                    println!("{:?}", v);
                    let new_elem = Pos {
                        x: v[0],
                        y: v[1],
                        theta: v[2],
                    };
                    self.position_histogram.push(new_elem);

                    let new_elem = Pos {
                        x: v[0] + 0.235 * v[2].cos(),
                        y: v[1] + 0.235 * v[2].sin(),
                        theta: v[2],
                    };

                    println!("{:?}", new_elem);

                    self.position_histogram_front.push(new_elem);
                }
            }
        }

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
                ui.radio_value(&mut self.view, View::OdoTracking, "Odometry");
                ui.radio_value(&mut self.view, View::ArmControl, "Arm Control");
                ui.radio_value(&mut self.view, View::LidarTuning, "Lidar Tuning");
            });

            match self.view {
                View::PIDTuning => {
                    let error: PlotPoints = (0..self.pid_histogram.len())
                        .map(|i| [i as f64, self.pid_histogram.get(i).unwrap()[0]])
                        .collect();

                    let setpoint: PlotPoints = (0..self.pid_histogram.len())
                        .map(|i| [i as f64, self.pid_histogram.get(i).unwrap()[1]])
                        .collect();

                    let out: PlotPoints = (0..self.pid_histogram.len())
                        .map(|i| {
                            [
                                i as f64,
                                self.pid_histogram.get(i).unwrap()[2]
                                    + self.pid_histogram.get(i).unwrap()[3]
                                    + self.pid_histogram.get(i).unwrap()[4],
                            ]
                        })
                        .collect();

                    let out_p: PlotPoints = (0..self.pid_histogram.len())
                        .map(|i| [i as f64, self.pid_histogram.get(i).unwrap()[2]])
                        .collect();

                    let out_i: PlotPoints = (0..self.pid_histogram.len())
                        .map(|i| [i as f64, self.pid_histogram.get(i).unwrap()[3]])
                        .collect();

                    let out_d: PlotPoints = (0..self.pid_histogram.len())
                        .map(|i| [i as f64, self.pid_histogram.get(i).unwrap()[4]])
                        .collect();

                    let height = ui.available_height() * 0.3;

                    Plot::new("value plot").height(height).show(ui, |plot_ui| {
                        plot_ui.line(Line::new("Error", error));
                        plot_ui.line(Line::new("Setpoint", setpoint));
                    });

                    Plot::new("my_plot").height(height).show(ui, |plot_ui| {
                        plot_ui.line(Line::new("Output", out));
                        plot_ui.line(Line::new("Output P", out_p));
                        plot_ui.line(Line::new("Output I", out_i));
                        plot_ui.line(Line::new("Output D", out_d));
                    });

                    ui.horizontal(|ui| {
                        ui.radio_value(
                            &mut self.pid_target,
                            PIDTarget::EncoderMotor,
                            "Encoder Motor",
                        );
                        ui.radio_value(&mut self.pid_target, PIDTarget::DriveBase, "Drive Base");
                        ui.radio_value(&mut self.pid_target, PIDTarget::Shoulder, "Shoulder");
                    });

                    ui.horizontal(|ui| {
                        ui.label("kP");
                        ui.add(egui::DragValue::new(&mut self.kp).speed(0.001));
                    });

                    ui.horizontal(|ui| {
                        ui.label("kI");
                        ui.add(egui::DragValue::new(&mut self.ki).speed(0.001));
                    });

                    ui.horizontal(|ui| {
                        ui.label("kD");
                        ui.add(egui::DragValue::new(&mut self.kd).speed(0.001));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Max. Cumulative Error");
                        ui.add(egui::DragValue::new(&mut self.max_ce).speed(0.1));
                    });

                    if ui.button("Send PID Vals").clicked() {
                        let message = [
                            Code(PID),
                            Code(SET),
                            Code(pid_target_to_msg(self.pid_target.clone())),
                            F32(self.kp),
                            F32(self.ki),
                            F32(self.kd),
                            F32(self.max_ce),
                        ];

                        if let Some(port) = self.port.as_mut() {
                            send_message(port, &message);
                        }
                    }

                    ui.horizontal(|ui| {
                        ui.label("Base Speed");
                        ui.add(egui::DragValue::new(&mut self.base_speed).speed(0.01));
                    });

                    if ui
                        .button(format!("Tape following: {}", self.tape_following))
                        .clicked()
                    {
                        self.tape_following = !self.tape_following;
                    }

                    if ui.button("Send Speed and Tape Following").clicked() {
                        // Send tape following message.
                        println!("Sending speed and tape following message...");
                        let message = vec![
                            Code(DRIVE_BASE),
                            Code(SET),
                            F32(self.base_speed),
                            U32(self.tape_following as u32),
                        ];

                        if let Some(port) = self.port.as_mut() {
                            send_message(port, &message);
                        }
                    }
                }
                View::OdoTracking => {
                    let pos1: PlotPoints = (0..self.position_histogram.len())
                        // TODO: add plot angle as rotation matrix.
                        .map(|i| {
                            [
                                self.position_histogram[i].x as f64,
                                self.position_histogram[i].y as f64,
                            ]
                        })
                        .collect();

                    let pos2: PlotPoints = (0..self.position_histogram_front.len())
                        .map(|i| {
                            [
                                self.position_histogram_front[i].x as f64,
                                self.position_histogram_front[i].y as f64,
                            ]
                        })
                        .collect();

                    let line1 = Line::new("Back Position", pos1);
                    let line2 = Line::new("Front Position", pos2);

                    let plot_height = ui.available_height() * 0.8;

                    Plot::new("Position Plot")
                        // .view_aspect(1.0)
                        .data_aspect(1.0)
                        .height(plot_height)
                        .legend(egui_plot::Legend::default())
                        .show(ui, |plot_ui| {
                            plot_ui.line(line1);
                            plot_ui.line(line2);
                        });

                    if ui.button("Erase Path").clicked() {
                        self.position_histogram.clear();
                        self.position_histogram_front.clear();
                    }

                    ui.horizontal(|ui| {
                        ui.label("Plot Angle");
                        ui.add(
                            egui::DragValue::new(&mut self.position_plot_angle)
                                .speed(0.5)
                                .range(0.0..=360.0),
                        );
                    });
                }
                View::ArmControl => {
                    for event in ctx.input(|i| i.events.clone()) {
                        if let egui::Event::MouseWheel {
                            unit: _,
                            delta,
                            modifiers: _,
                        } = event
                        {
                            println!("Mouse scrolled by: {}", delta.y);

                            let message =
                                vec![Code(TTBL), Code(SET), F32(delta.y * self.ttbl_sensitivity)];
                        }
                    }

                    let plot_height = ui.available_height() * 0.8;

                    Plot::new("Arm Plot")
                        .height(plot_height)
                        .data_aspect(1.0)
                        .view_aspect(1.0)
                        .include_x(-1.0)
                        .include_x(18.0)
                        .include_y(-1.0)
                        .include_y(18.0)
                        .allow_double_click_reset(false)
                        .allow_drag(false)
                        .allow_zoom(false)
                        .allow_boxed_zoom(false)
                        .allow_scroll(false)
                        .allow_axis_zoom_drag(false)
                        .auto_bounds(eframe::egui::Vec2b { x: false, y: false })
                        .legend(egui_plot::Legend::default())
                        .show(ui, |plot_ui| {
                            if let Some(mouse_pos) = plot_ui.pointer_coordinate() {
                                let response = plot_ui.response();
                                if response.is_pointer_button_down_on() || response.clicked() {
                                    self.arm_r = mouse_pos.x as f32;
                                    self.arm_h = mouse_pos.y as f32;

                                    let message = vec![
                                        Code(ARM),
                                        Code(SET),
                                        F32(self.arm_r),
                                        F32(self.arm_h),
                                    ];
                                }

                                let k = (((mouse_pos.y - 7.0).powf(2.0) + mouse_pos.x.powf(2.0)
                                    - 128.0)
                                    / 128.0)
                                    .acos();
                                let l = ((mouse_pos.y - 7.0) / mouse_pos.x).atan() + 0.5 * k;

                                plot_ui.line(
                                    Line::new(
                                        "arm ghost",
                                        vec![
                                            [0.0, 7.0],
                                            [8.0 * l.cos(), 8.0 * l.sin() + 7.0],
                                            [
                                                8.0 * (l.cos() + (l - k).cos()),
                                                8.0 * (l.sin() + (l - k).sin()) + 7.0,
                                            ],
                                            // [mouse_pos.x, mouse_pos.y]
                                        ],
                                    )
                                    .color(Color32::from_rgba_unmultiplied(0, 255, 255, 80)),
                                );

                                let k = (((self.arm_h as f64 - 7.0).powf(2.0)
                                    + (self.arm_r as f64).powf(2.0)
                                    - 128.0)
                                    / 128.0)
                                    .acos();
                                let l = (((self.arm_h as f64) - 7.0) / (self.arm_r as f64)).atan()
                                    + 0.5 * k;

                                plot_ui.line(Line::new(
                                    "current arm",
                                    vec![
                                        [0.0, 7.0],
                                        [8.0 * l.cos(), 8.0 * l.sin() + 7.0],
                                        [
                                            8.0 * (l.cos() + (l - k).cos()),
                                            8.0 * (l.sin() + (l - k).sin()) + 7.0,
                                        ],
                                        // [mouse_pos.x, mouse_pos.y]
                                    ],
                                ));
                            }
                        });

                    ui.horizontal(|ui| {
                        ui.label("TTbl Sensitivity");
                        ui.add(egui::DragValue::new(&mut self.ttbl_sensitivity).speed(1.0));
                    });

                    // println!("ASDSADA");
                }
                View::LidarTuning => {}
            }
        });
    }
}

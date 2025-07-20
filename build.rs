fn main() {
    let bindings = bindgen::builder()
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: true,
        })
        .header("../esp_serial_interface_test/include/serial_protocol.h")
        .generate()
        .expect("Unable to generate bindings :(");
    bindings
        .write_to_file("./src/serial_protocol.rs")
        .expect("Couldn't write bindings :(");
}

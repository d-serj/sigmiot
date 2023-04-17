fn main() {
    protobuf_codegen::Codegen::new()
        .cargo_out_dir("protos")
        .include("../")
        .input("../protos/sensors_data.proto")
        .run_from_script();
}

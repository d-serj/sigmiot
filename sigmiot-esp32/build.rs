// Necessary because of this issue: https://github.com/rust-lang/cargo/issues/9641
fn main() -> Result<(), Box<dyn std::error::Error>> {
    embuild::build::CfgArgs::output_propagated("ESP_IDF")?;
    embuild::build::LinkArgs::output_propagated("ESP_IDF")?;

    protobuf_codegen::Codegen::new()
        .cargo_out_dir("protos")
        .include("../")
        .input("../protos/sensors_data.proto")
        .run_from_script();

    Ok(())
}

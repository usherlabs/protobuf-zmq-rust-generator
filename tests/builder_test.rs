use std::io::Result;
use protobuf_zqm_rust_gen::ZmqServerGenerator;
use path_absolutize::*;

#[test]
fn test_building() -> Result<()> {
    // define PROTOC env as the path to protoc before running

    // let's error if it's not defined
    std::env::var("PROTOC").expect("PROTOC env var not defined");

    // relative file to ./generated
    let out_dir = std::path::Path::new("tests/generated/").absolutize()?;

    // print outdir
    println!("outdir: {:?}", out_dir);

    prost_build::Config::new()
        .out_dir(out_dir)
        .service_generator(Box::new(ZmqServerGenerator {}))
        .compile_protos(&["test_service.proto"], &["tests/assets/"])?;
    Ok(())
}

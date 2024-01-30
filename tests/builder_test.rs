use std::io::Result;

use path_absolutize::*;

use protobuf_zmq_rust_generator::ZmqServerGenerator;

#[test]
fn test_building() -> Result<()> {
    // define PROTOC env as the path to protoc before running

    // let's error if it's not defined
    std::env::var("PROTOC").expect("PROTOC env var not defined. You better have it installed and its path defined in PROTOC env var");

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

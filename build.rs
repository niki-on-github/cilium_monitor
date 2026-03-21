fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile the observer and flow protobufs into Rust modules
    // observer.proto imports flow/flow.proto and relay/relay.proto via 'import public'
    tonic_build::configure().build_server(false).compile(
        &[
            "proto/observer/observer.proto",
            "proto/flow/flow.proto",
            "proto/relay/relay.proto",
        ],
        &["proto/"],
    )?;
    Ok(())
}

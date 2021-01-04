use std::io::Write;

use schemars::schema_for;

macro_rules! gen_for {
    ($file_name:expr, $typ:ty) => {{
        let schema = schema_for!($typ);
        let file = std::fs::File::create($file_name).unwrap();
        let buf = std::io::BufWriter::new(file);
        serde_json::to_writer_pretty(buf, &schema).unwrap();
    }};
}

fn main() {
    std::fs::create_dir_all("schemas").unwrap();

    gen_for!("schemas/orch_client.jsonschema", livecore_protocol::OrchClientMsg);
    gen_for!("schemas/orch_server.jsonschema", livecore_protocol::OrchServerMsg);
}

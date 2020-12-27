use std::sync::Mutex;

use gstreamer as gst;
use gstreamer_app as gst_app;

use futures::sink::SinkExt;
use futures::stream::StreamExt;

use gst::prelude::*;

mod h264_fmp4_sink;
mod mp4_dumper;

fn video_gen() -> h264_fmp4_sink::Mp4Stream {
    gst::init().unwrap();

    let main_loop = glib::MainLoop::new(None, false);

    let pipeline = gst::Pipeline::new(None);

    let src = gst::ElementFactory::make("videotestsrc", None).expect("missing videotestsrc");
    src.set_property("is_live", &true).unwrap();

    let overlay = gst::ElementFactory::make("timeoverlay", None).expect("missing timeoverlay");

    let caps = gst::Caps::builder("video/x-raw")
        .field("width", &1280i32)
        .field("height", &720i32)
        .field("framerate", &gst::Fraction::new(30, 1))
        .build();
    let capsfilter = gst::ElementFactory::make("capsfilter", None).unwrap();
    capsfilter.set_property("caps", &caps).unwrap();

    //let src = gst::ElementFactory::make("srtsrc", None).expect("missing srtscr element");
    //src.set_property("uri", &"srt://0.0.0.0:7001?mode=listener").unwrap();

    let enc = gst::ElementFactory::make("x264enc", None).expect("missing x264enc");
    enc.set_property("key-int-max", &(30u32 * 2)).unwrap();

    //let mux = gst::ElementFactory::make("mpegtsmux", None).expect("missing mpegtsmux");

    let sink = gst::ElementFactory::make("appsink", None).expect("missing appsink");

    //let sink = gst::ElementFactory::make("udpsink", None).expect("missing udpsink");
    //sink.set_property("host", &"127.0.0.1").unwrap();
    //sink.set_property("port", &10004).unwrap();

    //let fpsdisplay =
    //    gst::ElementFactory::make("fpsdisplaysink", None).expect("missing fpsdisplaysink");
    //fpsdisplay.set_property("sync", &false).unwrap();

    pipeline
        .add_many(&[&src, &overlay, &capsfilter, &enc, &sink])
        .unwrap();
    gst::Element::link_many(&[&src, &overlay, &capsfilter, &enc, &sink]).unwrap();
    //gst::Element::link(&mux, &fpsdisplay).unwrap();

    let appsink = sink
        .dynamic_cast::<gst_app::AppSink>()
        .expect("sink element expected to be appsink");

    let mp4_stream = h264_fmp4_sink::setup(&appsink);

    pipeline.set_state(gst::State::Playing).unwrap();

    std::thread::spawn(move || {
        let bus = pipeline
            .get_bus()
            .expect("Pipeline without bus. Shouldn't happen!");

        for msg in bus.iter_timed(gst::CLOCK_TIME_NONE) {
            use gst::MessageView;

            match msg.view() {
                MessageView::Eos(..) => break,
                MessageView::Error(err) => {
                    pipeline.set_state(gst::State::Null).unwrap();
                    panic!(
                        "src: {}\nerror: {}\ndebug: {}\nsource: {}",
                        msg.get_src()
                            .map(|s| String::from(s.get_path_string()))
                            .unwrap_or_else(|| String::from("None")),
                        err.get_error().to_string(),
                        err.get_debug().unwrap_or("".into()),
                        err.get_error()
                    );
                }
                _ => (),
            }
        }
    });

    //pipeline.set_state(gst::State::Null).unwrap();
    mp4_stream
}

async fn accept_connection(
    mut mp4_stream: h264_fmp4_sink::Mp4Stream,
    stream: tokio::net::TcpStream,
) {
    use livecore_protocol::{serialize, Message};

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during websocket handshake");

    println!("new websocket connection");

    let (mut write, read) = ws_stream.split();

    let init_segment = mp4_stream.get_init().await;
    let message = Message::StreamData { data: init_segment };
    let serialized: Vec<u8> = serialize(&message).unwrap();
    write
        .send(tokio_tungstenite::tungstenite::Message::Binary(serialized))
        .await
        .unwrap();

    let receiver = mp4_stream.subscribe();
    receiver
        .into_stream()
        .map(|data| {
            let message = Message::StreamData {
                data: data.unwrap(),
            };
            let serialized: Vec<u8> = serialize(&message).unwrap();
            Ok(tokio_tungstenite::tungstenite::Message::Binary(serialized))
        })
        .forward(write)
        .await
        .unwrap();

    //read.forward(write).await.unwrap();
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();
    println!("listening on: {}", "127.0.0.1:8080");

    let mp4_stream = video_gen();

    //let _dumper_handle = tokio::spawn(mp4_dumper::dump_stream(mp4_stream.clone()));

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(accept_connection(mp4_stream.clone(), stream));
    }

    ()
}

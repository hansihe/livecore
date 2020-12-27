use crate::gst;
use crate::gst_app;

use std::sync::Mutex;

use gst::prelude::*;
use gst::Buffer;

use byteorder::{BigEndian, ReadBytesExt};
use once_cell::sync::OnceCell;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::sync::Arc;

use tokio::sync::{broadcast, watch};

#[derive(Debug, PartialEq, Eq)]
#[repr(u8)]
enum NalUnitType {
    Unspecified = 0,
    NonIDRPictureCodedSlice = 1,
    PartitionACodedSlice = 2,
    PartitionBCodedSlice = 3,
    PartitionCCodedSlice = 4,
    IDRPictureCodedSlice = 5,
    SupplementalEnhancementInformation = 6,
    SequenceParameterSet = 7,
    PictureParameterSet = 8,
    AccessUnitDelimiter = 9,
    EndOfSequence = 10,
    EndOfStream = 11,
    FillerData = 12,
    SequenceParameterSetExt = 13,
    PrefixNALUnit = 14,
    SubsetSequenceParameterSet = 15,
    DepthParameterSet = 16,
    SliceLayerWithoutPartitioning = 19,
    CodedSliceExtension = 20,
    SliceLayerExtension = 21,
    Unknown = 255,
}

#[derive(Debug, Clone)]
pub struct Mp4Stream {
    init: watch::Receiver<Option<Vec<u8>>>,
    media: Arc<broadcast::Sender<Vec<u8>>>,
}

impl Mp4Stream {
    pub async fn get_init(&mut self) -> Vec<u8> {
        loop {
            {
                let borrowed = self.init.borrow();
                if let Some(init) = &*borrowed {
                    return init.clone();
                }
            }
            self.init
                .changed()
                .await
                .expect("failure to get init segment, sender dropped");
        }
    }
    pub fn subscribe(&self) -> broadcast::Receiver<Vec<u8>> {
        self.media.subscribe()
    }
}

struct State {
    frame_number: usize,
    chunk_number: usize,
    start_dec_time: usize,
    framerate_denom: Option<u32>,
    collected_frames: Vec<Vec<u8>>,
}

pub fn setup(appsink: &gst_app::AppSink) -> Mp4Stream {
    let (init_sender, init_receiver) = watch::channel(None);

    let (media_sender_inner, _media_receiver) = broadcast::channel(4);
    let media_sender = Arc::new(media_sender_inner);

    let stream = Mp4Stream {
        init: init_receiver.clone(),
        media: media_sender.clone(),
    };

    appsink.set_caps(Some(&gst::Caps::new_simple(
        "video/x-h264",
        &[("stream-format", &"avc")],
    )));

    let state = Mutex::new(State {
        frame_number: 0,
        chunk_number: 0,
        start_dec_time: 0,
        framerate_denom: None,
        collected_frames: Vec::new(),
    });

    appsink.set_callbacks(
        gst_app::AppSinkCallbacks::builder()
            .new_sample(move |appsink| {
                let mut state = state.lock().unwrap();

                let sample = appsink.pull_sample().map_err(|_| gst::FlowError::Eos)?;

                if init_receiver.borrow().is_none() {
                    let caps = sample.get_caps().unwrap();
                    let caps_structure = caps.get_structure(0).unwrap();

                    let width: i32 = caps_structure.get_some("width").unwrap();
                    let height: i32 = caps_structure.get_some("height").unwrap();

                    let framerate: gst::Fraction =
                        caps_structure.get("framerate").unwrap().unwrap();

                    let codec_data: gst::Buffer =
                        caps_structure.get("codec_data").unwrap().unwrap();
                    let map = codec_data.map_readable().unwrap();
                    let slice = &map.as_slice()[1..4];

                    let init = livecore_mp4::StreamInit {
                        width: width as u32,
                        height: height as u32,
                        timescale: *framerate.numer() as u32,

                        avc_profile_indication: slice[0],
                        profile_compatibility: slice[1],
                        avc_level_indication: slice[2],
                    };

                    let mut buf = Vec::new();
                    init.write(&mut buf).unwrap();

                    {
                        let f = std::fs::File::create("frameout/init.mp4").unwrap();
                        let bw = std::io::BufWriter::new(f);
                        init.write(bw).unwrap();
                    }

                    init_sender.send(Some(buf)).unwrap();
                    state.framerate_denom = Some(*framerate.denom() as u32);

                    log::info!(
                        "wrote mp4 init segment according to caps {:?}",
                        caps_structure
                    );
                }

                let buffer = sample.get_buffer().ok_or_else(|| {
                    gst::gst_element_error!(
                        appsink,
                        gst::ResourceError::Failed,
                        ("Failed to get buffer from appsink")
                    );

                    gst::FlowError::Error
                })?;

                let map = buffer.map_readable().map_err(|_| {
                    gst::gst_element_error!(
                        appsink,
                        gst::ResourceError::Failed,
                        ("Failed to map buffer readable")
                    );

                    gst::FlowError::Error
                })?;

                let data: Vec<u8> = map.as_slice().into();

                let mut has_idr = false;

                let mut cur = std::io::Cursor::new(&data[..]);
                while let Ok(len) = cur.read_u32::<BigEndian>() {
                    let pos = cur.seek(SeekFrom::Current(0)).unwrap();
                    let head_byte = cur.read_u8().unwrap();

                    use NalUnitType as NUT;
                    let nal_unit_type = match head_byte & 0b11111 {
                        0 => NUT::Unspecified,
                        1 => NUT::NonIDRPictureCodedSlice,
                        2 => NUT::PartitionACodedSlice,
                        3 => NUT::PartitionBCodedSlice,
                        4 => NUT::PartitionCCodedSlice,
                        5 => NUT::IDRPictureCodedSlice,
                        6 => NUT::SupplementalEnhancementInformation,
                        7 => NUT::SequenceParameterSet,
                        8 => NUT::PictureParameterSet,
                        9 => NUT::AccessUnitDelimiter,
                        10 => NUT::EndOfSequence,
                        11 => NUT::EndOfStream,
                        12 => NUT::FillerData,
                        13 => NUT::SequenceParameterSetExt,
                        14 => NUT::PrefixNALUnit,
                        15 => NUT::SubsetSequenceParameterSet,
                        16 => NUT::DepthParameterSet,
                        19 => NUT::SliceLayerWithoutPartitioning,
                        20 => NUT::CodedSliceExtension,
                        21 => NUT::SliceLayerExtension,
                        _ => NUT::Unknown,
                    };

                    if nal_unit_type == NUT::IDRPictureCodedSlice {
                        has_idr = true;
                    }

                    log::debug!(
                        "NAL {:?} ({}) {:#08b} len:{}",
                        nal_unit_type,
                        head_byte & 0b11111,
                        head_byte,
                        len
                    );

                    //if nal_unit_type == NUT::SupplementalEnhancementInformation {
                    //    let mut payload_type = 0;
                    //    let mut b = cur.read_u8().unwrap();
                    //    while b == 0xff {
                    //        payload_type += 0xff;
                    //        b = cur.read_u8().unwrap();
                    //    }
                    //    payload_type += b as usize;

                    //    let mut payload_size = 0;
                    //    let mut b = cur.read_u8().unwrap();
                    //    while b == 0xff {
                    //        payload_size += 0xff;
                    //        b = cur.read_u8().unwrap();
                    //    }
                    //    payload_size += b as usize;

                    //    println!("SEI: type:{} len:{}", payload_type, payload_size);

                    //    let pos = cur.seek(SeekFrom::Current(0)).unwrap() as usize;
                    //    let sei_data = &data[pos..(pos + payload_size)];

                    //    // User data
                    //    if payload_type == 5 {
                    //        let string = String::from_utf8_lossy(sei_data);
                    //        println!("User data: {}", string);
                    //    }

                    //    //let pos = cur.seek(SeekFrom::Current(0)).unwrap() as usize;
                    //    //println!("{:?}", &data[pos..(pos + 10)]);
                    //}

                    cur.seek(SeekFrom::Start(pos + len as u64)).unwrap();
                }

                if has_idr && state.collected_frames.len() != 0 {
                    let stream_data = livecore_mp4::StreamData {
                        sequence_number: state.chunk_number as u32,
                        track_id: 1,

                        start_decode_time: state.start_dec_time as u64,

                        starts_with_idr: has_idr,
                        sample_duration: state.framerate_denom.unwrap(),

                        samples: state
                            .collected_frames
                            .iter()
                            .map(|c| livecore_mp4::StreamSample { data: c })
                            .collect(), //samples: vec![livecore_mp4::StreamSample { data: &data }],
                    };

                    {
                        let mut buf = Vec::new();
                        let cur = Cursor::new(&mut buf);
                        stream_data.write(cur).unwrap();
                        let _ = media_sender.send(buf);
                    }

                    //{
                    //    let f = std::fs::File::create(format!(
                    //        "frameout/chunk_{}.mp4",
                    //        state.chunk_number
                    //    ))
                    //    .unwrap();
                    //    let bw = std::io::BufWriter::new(f);
                    //    stream_data.write(bw).unwrap();
                    //}

                    state.collected_frames.clear();
                    state.chunk_number += 1;
                }

                if state.collected_frames.len() == 0 {
                    state.start_dec_time = state.frame_number;
                }
                state.collected_frames.push(data);

                state.frame_number += 1;

                Ok(gst::FlowSuccess::Ok)
            })
            .build(),
    );

    stream
}

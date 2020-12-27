use mp4::mp4box::{
    AvcCBox, AvcNBox, BoxHeader, BoxType, DinfBox, DrefBox, FtypBox, HdlrBox, MdhdBox, MdiaBox,
    MehdBox, MfhdBox, MinfBox, MoofBox, MoovBox, Mp4Box, MvexBox, MvhdBox, StblBox, StcoBox,
    StscBox, StsdBox, StszBox, SttsBox, TfdtBox, TfhdBox, TkhdBox, TrafBox, TrakBox, TrexBox,
    TrunBox, UrlBox, VmhdBox, WriteBox, HEADER_SIZE,
};
use mp4::{FixedPointU16, FixedPointU32, SampleFlags};
use std::convert::TryInto;
use std::error::Error;

pub struct StreamInit {
    pub width: u32,
    pub height: u32,
    // 1/timescale is the unit time used for the track.
    pub timescale: u32,

    pub avc_profile_indication: u8,
    pub profile_compatibility: u8,
    pub avc_level_indication: u8,
}

impl StreamInit {
    pub fn write<W: std::io::Write>(&self, mut writer: W) -> Result<(), Box<dyn Error>> {
        let finf = FtypBox {
            major_brand: "isom".into(),
            minor_version: 512,
            compatible_brands: vec!["isom".into(), "iso5".into(), "avc3".into(), "mp4l".into()],
        };
        finf.write_box(&mut writer)?;
        println!("{}", finf.get_size());

        let moov = MoovBox {
            mvhd: MvhdBox {
                version: 0,
                flags: 0,

                creation_time: 0,
                modification_time: 0,

                timescale: self.timescale,
                // DASH-IF 10.2.3.3. The duration field in the mvhd box shall be
                // set to 0
                duration: 0,
                rate: FixedPointU32::new_whole(1),
                volume: FixedPointU16::new_whole(0),
                matrix: Default::default(),
                next_track_id: 0,
            },
            traks: vec![TrakBox {
                tkhd: TkhdBox {
                    version: 0,
                    flags: 0x01 // track_enabled
                            | 0x02 // track_in_movie
                            | 0x04, // track_in_preview
                    //| 0x08 // track_size_is_aspect_ratio
                    creation_time: 0,
                    modification_time: 0,
                    track_id: 1,
                    // 14496-12 8.3.2.3: If the duration of this track
                    // cannot be determined then duration is set to all 1s.
                    duration: !0,
                    layer: 0,
                    alternate_group: 0,
                    volume: FixedPointU16::new_whole(0),
                    matrix: Default::default(),
                    width: FixedPointU32::new_whole(self.width),
                    height: FixedPointU32::new_whole(self.height),
                },
                mdia: MdiaBox {
                    mdhd: MdhdBox {
                        version: 0,
                        flags: 0,
                        creation_time: 0,
                        modification_time: 0,

                        // 14496-12 8.4.2.3: If the duration cannot be determined then duration is set to all 1s.
                        duration: !0,
                        timescale: self.timescale,

                        language: "eng".into(),
                    },
                    hdlr: HdlrBox {
                        version: 0,
                        flags: 0,

                        handler_type: "vide".into(),
                        name: "LiveCore fMP4 builder".into(),
                    },
                    minf: MinfBox {
                        vmhd: Some(VmhdBox {
                            version: 0,
                            flags: 0,
                            graphics_mode: 0,
                            op_color: (0, 0, 0).into(),
                        }),
                        smhd: None,
                        dinf: DinfBox {
                            dref: DrefBox {
                                version: 0,
                                flags: 0,
                                data_entries: vec![UrlBox {
                                    version: 0,
                                    // 0x01: media data exists in same file
                                    flags: 0x01,

                                    location: "".into(),
                                }],
                            },
                        },
                        stbl: StblBox {
                            stsd: StsdBox {
                                version: 0,
                                flags: 0,

                                avc1: None,
                                avc2: None,
                                avc3: Some(AvcNBox {
                                    variant: Default::default(),

                                    data_reference_index: 1,
                                    width: self.width.try_into().unwrap(),
                                    height: self.height.try_into().unwrap(),
                                    horizresolution: FixedPointU32::new_whole(72),
                                    vertresolution: FixedPointU32::new_whole(72),
                                    // Frames per sample
                                    frame_count: 1,
                                    // 0x18: color with no alpha
                                    depth: 0x18,
                                    avcc: AvcCBox {
                                        // 14496-15 5.2.4.1
                                        configuration_version: 1,

                                        // Should be replicated from SPS NAL.
                                        // ITU-T H.264 7.3.2.1.1 Sequence parameter set data syntax
                                        // 14496-15 A.2.1 Baseline Profile
                                        //avc_profile_indication: 66,
                                        //profile_compatibility: 192,
                                        //avc_level_indication: 21,
                                        avc_profile_indication: self.avc_profile_indication,
                                        profile_compatibility: self.profile_compatibility,
                                        avc_level_indication: self.avc_level_indication,

                                        // Size of NAL AVCC length prefix.
                                        // 4 - 1 = 3 (N - 1)
                                        length_size_minus_one: 3,

                                        // type is avc3, these are inline in NAL stream
                                        sequence_parameter_sets: vec![],
                                        picture_parameter_sets: vec![],
                                    },
                                }),
                                hev1: None,
                                mp4a: None,
                                tx3g: None,
                            },
                            stts: SttsBox {
                                version: 0,
                                flags: 0,
                                entries: vec![],
                            },
                            stsc: StscBox {
                                version: 0,
                                flags: 0,
                                entries: vec![],
                            },
                            stsz: StszBox {
                                version: 0,
                                flags: 0,
                                sample_size: 0,
                                sample_count: 0,
                                sample_sizes: vec![],
                            },
                            stco: Some(StcoBox {
                                version: 0,
                                flags: 0,
                                entries: vec![],
                            }),

                            co64: None,
                            ctts: None,
                            stss: None,
                        },
                    },
                },
                edts: None,
            }],
            mvex: Some(MvexBox {
                // 14496-12 8.8.2.3: If an MP4 file is created in real-time,
                // such as used in live streaming, it is not likely that the
                // fragment_duration is known in advance and this box may be
                // omitted.
                mehd: None,
                trex: TrexBox {
                    version: 0,
                    flags: 0,

                    track_id: 1,
                    default_sample_description_index: 1,
                    default_sample_duration: 0,
                    default_sample_size: 0,
                    default_sample_flags: {
                        let mut flags = SampleFlags::default();
                        flags.is_non_sync = true;
                        flags
                    },
                },
            }),
        };
        moov.write_box(&mut writer)?;
        println!("{}", moov.get_size());

        Ok(())
    }
}

pub struct StreamData<'a> {
    pub sequence_number: u32,
    pub track_id: u32,

    pub start_decode_time: u64,

    pub starts_with_idr: bool,
    pub sample_duration: u32,

    pub samples: Vec<StreamSample<'a>>,
}

pub struct StreamSample<'a> {
    pub data: &'a [u8],
}

impl<'a> StreamData<'a> {
    pub fn write<W: std::io::Write>(&self, mut writer: W) -> Result<(), Box<dyn Error>> {
        let mut moof = MoofBox {
            mfhd: MfhdBox {
                version: 0,
                flags: 0,

                sequence_number: self.sequence_number,
            },
            trafs: vec![TrafBox {
                tfhd: TfhdBox {
                    version: 0,
                    track_id: self.track_id,

                    // inherited from mvex
                    sample_description_index: None,

                    // overridden by trun
                    default_sample_size: None,
                    default_sample_flags: None,

                    default_sample_duration: Some(self.sample_duration),

                    // DASH-if 3.2.1: default-base-is-moof
                    default_base_is_moof: true,
                    // DASH-IF 3.2.1: base-data-offset shall not be used.
                    base_data_offset: None,
                    duration_is_empty: false,
                },
                tfdt: Some(TfdtBox {
                    version: 1,
                    base_media_decode_time: self.start_decode_time,
                }),
                trun: Some(TrunBox {
                    version: 0,
                    sample_count: self.samples.len() as u32,
                    data_offset: Some(123),
                    first_sample_flags: Some({
                        let mut flags = SampleFlags::default();
                        flags.is_non_sync = !self.starts_with_idr;
                        flags
                    }),

                    sample_sizes: Some(self.samples.iter().map(|s| s.data.len() as u32).collect()),
                    sample_durations: Some(self.samples.iter().map(|_| 1).collect()),

                    sample_flags: None,
                    sample_composition_time_offsets: None,
                }),
            }],
        };

        let moof_size = moof.box_size();
        moof.trafs[0].trun.as_mut().unwrap().data_offset = Some(moof_size as i32 + 8);

        moof.write_box(&mut writer)?;

        let combined_sample_size: usize = self.samples.iter().map(|s| s.data.len()).sum();
        BoxHeader::new(BoxType::MdatBox, HEADER_SIZE + combined_sample_size as u64)
            .write(&mut writer)?;

        for sample in self.samples.iter() {
            writer.write(&sample.data)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Seek;

    #[test]
    fn write_basic() {
        let init = StreamInit {
            width: 10,
            height: 10,
            timescale: 60,

            avc_profile_indication: 66,
            profile_compatibility: 192,
            avc_level_indication: 21,
        };

        let mut f = std::fs::File::create("yay.mp4").unwrap();
        init.write(&mut f).unwrap();

        let pos = f.seek(std::io::SeekFrom::Current(0)).unwrap();
        println!("{}", pos);
    }
}

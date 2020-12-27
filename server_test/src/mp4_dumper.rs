use tokio::fs;
use tokio::io::{AsyncWriteExt, BufWriter};

pub async fn dump_stream(mut stream: crate::h264_fmp4_sink::Mp4Stream) {
    {
        let init_segment = stream.get_init().await;

        let path = "frameout/init.mp4";
        log::info!("dumping init segment: {}", path);

        let mut file = fs::File::create(path).await.unwrap();
        file.write(&init_segment).await.unwrap();
    }

    let mut seg = 0;

    let mut subscription = stream.subscribe();

    let mut item = subscription.recv().await;
    while let Ok(media_segment) = item {
        let path = format!("frameout/chunk_{}.mp4", seg);
        log::info!("dumping media segment {}: {}", seg, path);

        let mut file = fs::File::create(&path).await.unwrap();
        {
            let mut writer = BufWriter::new(&mut file);
            writer.write_all(&media_segment).await.unwrap();
            writer.flush().await.unwrap();
        }
        file.flush().await.unwrap();

        seg += 1;

        item = subscription.recv().await;
    }
}

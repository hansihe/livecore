
async function top() {
    let client = await import("../../client_test/pkg/");
    client.init();

    let fabric = new client.Fabric("ws://localhost:8080");
    console.log("Fabric created");

    let video = document.querySelector('video');
    window.thevid = video;

    if (!window.MediaSource) {
        console.error('No Media Source API available');
        return;
    }

    let ms = new MediaSource();
    video.src = window.URL.createObjectURL(ms);

    let sourceBuffer;
    let segmentQueue = [];

    let msOpen = false;
    let processing = false;

    function nextSegment() {
        if (segmentQueue.length > 0) {
            processing = true;
            let seg = segmentQueue.shift();
            console.log("APPENDING: ", seg);
            sourceBuffer.appendBuffer(seg);
        } else {
            processing = false;
        }
    }

    function onMediaSourceOpen() {
        sourceBuffer = ms.addSourceBuffer('video/mp4; codecs="avc3.42E01E"');
        sourceBuffer.mode = 'sequence';

        sourceBuffer.addEventListener('updateend', nextSegment);

        msOpen = true;
        if (segmentQueue.length > 0) {
            processing = true;
            let seg = segmentQueue.shift();
            console.log("APPENDING: ", seg);
            sourceBuffer.appendBuffer(seg);
        }

        //try {
        //    video.play();
        //} catch (e) {
        //    console.warn("PLAY FAILED", e);
        //}
    }
    ms.addEventListener('sourceopen', onMediaSourceOpen);

    fabric.tmp_set_got_segment_cb(data => {
        console.log(data);
        segmentQueue.push(data);

        if (msOpen && !processing) {
            processing = true;
            let seg = segmentQueue.shift();
            console.log("APPENDING: ", seg);
            sourceBuffer.appendBuffer(seg);
        }
    });
};

top();

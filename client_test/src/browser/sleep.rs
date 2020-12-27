use js_sys::Promise;
use wasm_bindgen_futures::JsFuture;
use web_sys::window;

pub async fn sleep(ms: i32) -> () {
    let promise = Promise::new(&mut |resolve, _| {
        let win = window().unwrap();
        win.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
            .unwrap();
    });
    let js_fut = JsFuture::from(promise);
    js_fut.await.unwrap();
}

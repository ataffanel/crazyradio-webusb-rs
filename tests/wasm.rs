use std::convert::TryInto;

use wasm_bindgen_test::*;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

use crazyradio_webusb::{Crazyradio, SharedCrazyradio};

#[wasm_bindgen_test]
async fn test_open_first() {
    let _ = Crazyradio::open_first_async().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_scan() {
    let radio = Crazyradio::open_first_async().await.unwrap();
    let radio = SharedCrazyradio::new(radio);

    let found = radio
        .scan_async(
            0.try_into().unwrap(),
            80.try_into().unwrap(),
            [0xe7; 5],
            vec![0xff],
        )
        .await
        .unwrap();
    wasm_bindgen_test::console_log!("Found channels: {:?}", found);
}

#[wasm_bindgen_test]
async fn test_list_serial() {
    let serials = Crazyradio::list_serials_async().await.unwrap();

    wasm_bindgen_test::console_log!("Already paired serials: {:?}", serials);

    assert!(!serials.is_empty());
    assert!(serials[0].len() == 10);
}

#[wasm_bindgen_test]
async fn test_open_by_serial() {
    let serials = Crazyradio::list_serials_async().await.unwrap();

    assert!(!serials.is_empty());

    let _ = Crazyradio::open_by_serial_async(&serials[0]).await.unwrap();
}

#[wasm_bindgen_test]
async fn test_open_by_serial_fails_on_notfound() {
    let result = Crazyradio::open_by_serial_async("").await;

    assert!(matches!(result, Err(crazyradio_webusb::Error::NotFound)));
}

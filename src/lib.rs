//! # Crazyradio driver for Rust using the WebUSB API.
//!
//! This driver is intended to be used when targetting the web browser with Wasm.
//! It replicates the async API subset of the native [crazyradio crate](https://crates.io/crates/crazyradio).
//!
//! The main intention of this crate is to be used as a compile-time replacement
//! for the native [crazyradio](https://github.com/ataffanel/crazyradio-rs) crate in
//! the [crazyflie-link](https://github.com/ataffanel/crazyflie-link-rs) crate.
//! The async functions to create the crazyradio can be used and then
//! the [Crazyradio] object must be passed and used though the [SharedCrazyradio]
//! object:
//!
//! ``` no_run
//! # use crazyradio_webusb::{Crazyradio, SharedCrazyradio, Error};
//! # async fn test() -> Result<(), Error> {
//! let radio = Crazyradio::open_nth_async(0).await?;
//! let shared_radio = SharedCrazyradio::new(radio);
//! # Ok(())
//! # }
//! ```

use std::convert::TryInto;

use wasm_bindgen::{prelude::*, JsCast};

use wasm_bindgen_futures::JsFuture;

use futures_util::lock::Mutex;

type Result<T> = std::result::Result<T, Error>;

/// Ack status
#[derive(Default, Debug, Copy, Clone)]
pub struct Ack {
    /// At true if an ack packet has been received
    pub received: bool,
    /// Value of the nRF24 power detector when receiving the ack packet
    pub power_detector: bool,
    /// Number of time the packet was sent before an ack was received
    pub retry: usize,
    /// Length of the ack payload
    pub length: usize,
}

/// Shared Crazyradio
pub struct SharedCrazyradio {
    radio: Mutex<Crazyradio>,
}

impl SharedCrazyradio {
    pub fn new(radio: Crazyradio) -> Self {
        Self {
            radio: Mutex::new(radio),
        }
    }

    pub async fn send_packet_async(
        &self,
        channel: Channel,
        address: [u8; 5],
        payload: Vec<u8>,
    ) -> Result<(Ack, Vec<u8>)> {
        let mut radio = self.radio.lock().await;
        radio.set_channel_async(channel).await?;
        radio.set_address_async(&address).await?;
        radio.send_packet_async(payload.clone()).await
    }

    pub async fn scan_async(
        &self,
        start: Channel,
        stop: Channel,
        address: [u8; 5],
        payload: Vec<u8>,
    ) -> Result<Vec<Channel>> {
        let mut found = Vec::new();

        let start: u8 = start.into();
        let stop: u8 = stop.into();

        for channel in start..=stop {
            let mut radio = self.radio.lock().await;
            radio.set_address_async(&address).await?;
            radio.set_channel_async(channel.try_into()?).await?;
            let (ack, _) = radio.send_packet_async(payload.clone()).await?;
            if ack.received {
                found.push(channel.try_into()?);
            }
        }

        Ok(found)
    }
}

/// Crazyradio dongle
///
/// This struct only contains function to create a new `Crazyradio` object. It
/// needs to be passed to the [SharedCrazyradio] to be used.
pub struct Crazyradio {
    device: web_sys::UsbDevice,
    current_channel: Option<Channel>,
    current_address: Option<[u8; 5]>,
}

const SET_RADIO_CHANNEL: u8 = 0x01;
const SET_RADIO_ADDRESS: u8 = 0x02;

impl Crazyradio {
    pub async fn open_first_async() -> Result<Crazyradio> {
        Self::open_nth_async(0).await
    }

    pub async fn open_nth_async(nth: usize) -> Result<Crazyradio> {
        if nth != 0 {
            return Err(Error::InvalidArgument);
        }

        let window = web_sys::window().expect("No global 'window' exists!");
        let navigator: web_sys::Navigator = window.navigator();
        let usb = navigator.usb();

        let filter: serde_json::Value =
            serde_json::from_str(r#"{ "filters": [{ "vendorId": 6421 }] }"#).unwrap();
        let filter = JsValue::from_serde(&filter).unwrap();

        let devices: js_sys::Array = JsFuture::from(usb.get_devices()).await?.into();

        // Open radio if one is already paired and plugged
        // Otherwise ask the user to pair a new radio
        let device: web_sys::UsbDevice = if devices.length() > 0 {
            devices.get(0).dyn_into().unwrap()
        } else {
            JsFuture::from(usb.request_device(&filter.into()))
                .await?
                .dyn_into()
                .unwrap()
        };

        JsFuture::from(device.open()).await?;
        JsFuture::from(device.claim_interface(0)).await?;

        Ok(Self {
            device,
            current_channel: None,
            current_address: None,
        })
    }

    #[cfg(feature = "unstable")]
    pub async fn list_serials_async() -> Result<Vec<String>> {
        let window = web_sys::window().expect("No global 'window' exists!");
        let navigator: web_sys::Navigator = window.navigator();
        let usb = navigator.usb();

        let devices: js_sys::Array = JsFuture::from(usb.get_devices()).await?.into();

        let mut serials = Vec::new();

        for device in devices.iter() {
            let device = device.dyn_into::<web_sys::UsbDevice>().unwrap();
            if let Some(serial) = device.serial_number() {
                serials.push(serial);
            }
        }

        Ok(serials)
    }

    async fn set_address_async(&mut self, address: &[u8; 5]) -> Result<()> {
        if self.current_address != Some(*address) {
            let parameter = web_sys::UsbControlTransferParameters::new(
                0,
                web_sys::UsbRecipient::Device,
                SET_RADIO_ADDRESS,
                web_sys::UsbRequestType::Vendor,
                0,
            );

            let mut data = *address;
            let transfer = self
                .device
                .control_transfer_out_with_u8_array(&parameter, &mut data);

            let _ = JsFuture::from(transfer)
                .await?
                .dyn_into::<web_sys::UsbOutTransferResult>()
                .unwrap();

            self.current_address = Some(*address);
        }

        Ok(())
    }

    async fn set_channel_async(&mut self, channel: Channel) -> Result<()> {
        if self.current_channel != Some(channel) {
            let parameter = web_sys::UsbControlTransferParameters::new(
                0,
                web_sys::UsbRecipient::Device,
                SET_RADIO_CHANNEL,
                web_sys::UsbRequestType::Vendor,
                channel.into(),
            );

            let mut data = [];
            let transfer = self
                .device
                .control_transfer_out_with_u8_array(&parameter, &mut data);

            let _ = JsFuture::from(transfer)
                .await?
                .dyn_into::<web_sys::UsbOutTransferResult>()
                .unwrap();

            self.current_channel = Some(channel);
        }

        Ok(())
    }

    async fn send_packet_async(&self, packet: Vec<u8>) -> Result<(Ack, Vec<u8>)> {
        let mut packet = packet;
        JsFuture::from(self.device.transfer_out_with_u8_array(0x01, &mut packet)).await?;

        let data = JsFuture::from(self.device.transfer_in(0x01, 64))
            .await?
            .dyn_into::<web_sys::UsbInTransferResult>()
            .unwrap();

        let mut pk = Vec::new();
        for i in 1..data.data().unwrap().byte_length() {
            pk.push(data.data().unwrap().get_uint8(i));
        }

        let mut ack = Ack::default();
        if data.data().unwrap().get_uint8(0) != 0 {
            ack.received = true;
            ack.length = pk.len();
        }

        Ok((ack, pk))
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde_support", derive(Serialize))]
pub struct Channel(u8);

impl Channel {
    pub fn from_number(channel: u8) -> Result<Self> {
        if channel < 126 {
            Ok(Channel(channel))
        } else {
            Err(Error::InvalidArgument)
        }
    }
}

// U8 -> Channel is not safe so we only implement Channel -> U8
#[allow(clippy::from_over_into)]
impl Into<u8> for Channel {
    fn into(self) -> u8 {
        self.0
    }
}

// U16 -> Channel is not safe so we only implement Channel -> U16
#[allow(clippy::from_over_into)]
impl Into<u16> for Channel {
    fn into(self) -> u16 {
        self.0.into()
    }
}

impl TryInto<Channel> for u8 {
    type Error = Error;

    fn try_into(self) -> std::result::Result<Channel, Self::Error> {
        Channel::from_number(self)
    }
}

#[derive(thiserror::Error, Debug, Clone)]
pub enum Error {
    #[error("Crazyradio not found")]
    NotFound,
    #[error("Invalid arguments")]
    InvalidArgument,
    #[error("Crazyradio version not supported")]
    DongleVersionNotSupported,
    #[error("Browser error")]
    BrowserError(String),
}

impl From<JsValue> for Error {
    fn from(e: JsValue) -> Self {
        Self::BrowserError(format!("{:?}", e))
    }
}

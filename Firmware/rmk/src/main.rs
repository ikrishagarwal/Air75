#![no_main]
#![no_std]

mod bongocat_frames;
mod bongocat_renderer;

// RMK generates hardware setup, storage, macros, and the display task from TOML.
#[rmk::macros::rmk_keyboard]
mod keyboard {
    // RMK's USB transport reports laptop suspend/resume through the connection
    // status event. Bridge that state to the sleep event consumed by the
    // display processor so the OLED follows the host's lid state.
    #[register_processor(event)]
    fn usb_sleep_bridge() {
        #[rmk::macros::processor(subscribe = [::rmk::event::ConnectionStatusChangeEvent])]
        struct UsbSleepBridge;

        impl UsbSleepBridge {
            async fn on_connection_status_change_event(
                &mut self,
                event: ::rmk::event::ConnectionStatusChangeEvent,
            ) {
                let sleeping = event.0.usb == ::rmk::types::connection::UsbState::Suspended;
                ::rmk::event::publish_event(::rmk::event::SleepStateEvent::new(sleeping));
            }
        }

        UsbSleepBridge
    }
}

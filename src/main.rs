#![no_std]
#![no_main]

use core::str;

use cyw43_pio::{PioSpi, DEFAULT_CLOCK_DIVIDER};
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio;
use embassy_rp::pio::Pio;
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;

// for USB serial logging
use {embassy_rp::peripherals::USB, embassy_rp::usb, embassy_rp::usb::Driver, log};

use {defmt_rtt as _, embassy_time as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
});

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // region USB Log Setup (feature)
    {
        let driver = Driver::new(p.USB, Irqs);
        spawner.spawn(logger_task(driver)).unwrap();
    }
    // endregion

    // region Wireless Setup
    
    #[allow(unused_variables)]
    let (fw, clm, btfw) = {
        // IMPORTANT
        //
        // Download and make sure these files from https://github.com/embassy-rs/embassy/tree/main/cyw43-firmware
        // are available in `./examples/rp-pico-w`. (should be automatic)
        //
        // IMPORTANT
        //include firmware binary files for wireless module
        let fw = include_bytes!("../cyw43-firmware/43439A0.bin");
        // include the country location matrix
        let clm = include_bytes!("../cyw43-firmware/43439A0_clm.bin");
        // include bluetooth firmware
        let btfw: &'static [u8] = { 
            #[cfg(feature = "bluetooth")]
            { include_bytes!("../cyw43-firmware/43439A0_btfw.bin") }
            #[cfg(not(feature = "bluetooth"))]
            { &[] }
        };
        (fw, clm, btfw)
    };

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        DEFAULT_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH0,
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());

    #[cfg(feature = "bluetooth")]
    #[allow(unused_variables)]
    let (_net_device, bt_device, mut control, runner) =
        cyw43::new_with_bluetooth(state, pwr, spi, fw, btfw).await;
    #[cfg(not(feature = "bluetooth"))]
    let (_net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;

    spawner.spawn(cyw43_task(runner)).unwrap();
    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;
    // endregion


    let delay = Duration::from_secs(2);
    loop {
        control.gpio_set(0, true).await;
        log::info!("led on, scanning wifi devices");

        let mut scanner = control.scan(Default::default()).await;
        while let Some(bss) = scanner.next().await {
            if let Ok(ssid_str) = str::from_utf8(&bss.ssid) {
                log::info!("scanned {}, strength: {}", ssid_str, bss.rssi);
            }
        }
        drop(scanner);
        

        control.gpio_set(0, false).await;
        log::info!("led off, scan done");
        log::info!("");
        
        Timer::after(delay).await;
    }
}

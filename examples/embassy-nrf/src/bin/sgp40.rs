#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use embassy_nrf::config::Config;
use embassy_nrf::gpio::Output;
use embassy_nrf::twim::{self, Twim};

use embassy_executor::Spawner;
use embassy_nrf::{bind_interrupts, peripherals};
use embassy_time::{Delay, Duration, Timer};

use example_embassy_nrf as _;
use sensirion_async::sgp40::Sgp40;

bind_interrupts!(struct Irqs {
    SPIM0_SPIS0_TWIM0_TWIS0_SPI0_TWI0 => twim::InterruptHandler<peripherals::TWISPI0>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Config::default());
    let mut led = Output::new(
        p.P1_10,
        embassy_nrf::gpio::Level::Low,
        embassy_nrf::gpio::OutputDrive::Standard,
    );
    let twi = Twim::new(p.TWISPI0, Irqs, p.P0_12, p.P0_11, Default::default());
    let mut sensor = Sgp40::new(twi);

    loop {
        led.set_high();
        Timer::after(Duration::from_millis(200)).await;
        led.set_low();
        Timer::after(Duration::from_millis(200)).await;
        let serial_number = defmt::unwrap!(sensor.get_serial_number().await);
        defmt::warn!("S/N {:x}", serial_number);

        let measurement = defmt::unwrap!(sensor.read(40.0, 20.0, &mut Delay).await);
        defmt::warn!("data: {}", measurement);
    }
}

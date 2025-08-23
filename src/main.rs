#![no_std]
#![no_main]

use assign_resources::assign_resources;
use embassy_executor::Spawner;
use embassy_rp::config::Config;
use embassy_rp::peripherals::{self, I2C0};
use embassy_rp::{bind_interrupts, Peri};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    ADC_IRQ_FIFO => embassy_rp::adc::InterruptHandler;
    I2C0_IRQ => embassy_rp::i2c::InterruptHandler<I2C0>;
});

assign_resources! {
    adc: AdcRes {
        adc: ADC,
    },
    system: SystemRes {
        adc_tmp: ADC_TEMP_SENSOR,
        usb: PIN_24,
        vsys: PIN_29,
    },
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Config::default());
    let _ = split_resources! {p};
}

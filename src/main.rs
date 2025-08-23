#![no_std]
#![no_main]

use assign_resources::assign_resources;
use embassy_executor::Spawner;
use embassy_rp::config::Config;
use embassy_rp::i2c::{self, I2c};
use embassy_rp::peripherals::{self, I2C0, UART0, UART1};
use embassy_rp::uart::{self, BufferedInterruptHandler, BufferedUart};
use embassy_rp::{bind_interrupts, Peri};
use emc2101::EMC2101;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    ADC_IRQ_FIFO => embassy_rp::adc::InterruptHandler;
    I2C0_IRQ => embassy_rp::i2c::InterruptHandler<I2C0>;
    UART0_IRQ => BufferedInterruptHandler<UART0>;
    UART1_IRQ => BufferedInterruptHandler<UART1>;
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
    emc: EmcRes {
        sda: PIN_4,
        scl: PIN_5,
        i2c0: I2C0,
    },
    button: ButtonRes {
        btn: PIN_12,
    },
    fan: FanRes {
        pwr: PIN_16,
    },
    led: LedRes {
        led: PIN_25,
    },
    uart0: Uart0Res {
        tx: PIN_0,
        rx: PIN_1,
        uart: UART0,
    },
    uart1: Uart1Res {
        tx: PIN_8,
        rx: PIN_9,
        uart: UART1,
    },
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Config::default());
    let r = split_resources! {p};

    // init uart0
    static TX_BUF: StaticCell<[u8; 16]> = StaticCell::new();
    static RX_BUF: StaticCell<[u8; 16]> = StaticCell::new();
    let tx_buf = &mut TX_BUF.init([0; 16])[..];
    let rx_buf = &mut RX_BUF.init([0; 16])[..];
    let _uart0 = BufferedUart::new(r.uart0.uart, r.uart0.tx, r.uart0.rx, Irqs, tx_buf, rx_buf, uart::Config::default());

    // init emc driver
    let i2c0 = I2c::new_async(r.emc.i2c0, r.emc.scl, r.emc.sda, Irqs, i2c::Config::default());
    let mut emc = EMC2101::new(i2c0).unwrap();

    match emc.status() {
        Ok(status) => defmt::info!("emc status {:?}", status),
        Err(_) => todo!(),
    }

    loop {
        // implement application logic
        // read temperature from sensors
        // read temperature from uart0
        // read temperature from uart1
        // determine fan speed based on reading
        // set fan speed
        // repeat every second
        todo!();
    }
}

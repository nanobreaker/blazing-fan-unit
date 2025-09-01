#![no_std]
#![no_main]

use assign_resources::assign_resources;
use bounded_integer::BoundedU8;
use embassy_executor::Spawner;
use embassy_rp::config::Config;
use embassy_rp::gpio::{Input, Output, Pull};
use embassy_rp::i2c::{self, I2c};
use embassy_rp::peripherals::{self, I2C0, PIO0, UART0, UART1};
use embassy_rp::pio::Pio;
use embassy_rp::pio_programs::ws2812::{PioWs2812, PioWs2812Program};
use embassy_rp::uart::{self, BufferedInterruptHandler, BufferedUart};
use embassy_rp::{bind_interrupts, Peri};
use embassy_time::{Duration, Ticker};
use embedded_io_async::Read;
use emc2101::EMC2101;
use fugit::Rate;
use smart_leds::RGB8;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    ADC_IRQ_FIFO => embassy_rp::adc::InterruptHandler;
    I2C0_IRQ => embassy_rp::i2c::InterruptHandler<I2C0>;
    UART0_IRQ => BufferedInterruptHandler<UART0>;
    UART1_IRQ => BufferedInterruptHandler<UART1>;
    PIO0_IRQ_0 => embassy_rp::pio::InterruptHandler<PIO0>;
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
    pixel: NeoPixelRes {
        pio: PIO0,
        led: PIN_15,
        dma: DMA_CH0,
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

#[derive(PartialEq)]
enum Mode {
    Auto,
    Manual,
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Config::default());
    let r = split_resources! {p};

    // init neopixel
    let mut pio = Pio::new(r.pixel.pio, Irqs);
    let program = PioWs2812Program::new(&mut pio.common);
    let mut ws2812 = PioWs2812::<PIO0, 0, 2>::new(&mut pio.common, pio.sm0, r.pixel.dma, r.pixel.led, &program);

    // init uart0
    static TX_BUF: StaticCell<[u8; 16]> = StaticCell::new();
    static RX_BUF: StaticCell<[u8; 16]> = StaticCell::new();
    let tx_buf = &mut TX_BUF.init([0; 16])[..];
    let rx_buf = &mut RX_BUF.init([0; 16])[..];
    let _uart0 = BufferedUart::new(r.uart0.uart, r.uart0.tx, r.uart0.rx, Irqs, tx_buf, rx_buf, uart::Config::default());

    // init fan power
    let _fan_pwr = Output::new(r.fan.pwr, embassy_rp::gpio::Level::High);

    // init emc driver
    let i2c0 = I2c::new_async(r.emc.i2c0, r.emc.scl, r.emc.sda, Irqs, i2c::Config::default());
    let mut emc = EMC2101::new(i2c0).unwrap();
    emc.enable_tach_input().unwrap();
    emc.set_fan_pwm(Rate::<u32, _, _>::kHz(25), false).expect("should set fan pwm");

    match emc.status() {
        Ok(status) => defmt::info!("emc status {:?}", status),
        Err(_) => defmt::error!("emc not working"),
    }

    // init button
    let button = Input::new(r.button.btn, Pull::Up);

    let mut data = [RGB8::new(128, 0, 0); 2];
    let mut ticker = Ticker::every(Duration::from_secs(1));
    let mut mode = Mode::Auto;
    let mut power = <BoundedU8<0, 63>>::new(0).unwrap();

    loop {
        defmt::info!("tick passed");

        if button.is_low() {
            mode = match mode {
                Mode::Auto => Mode::Manual,
                Mode::Manual => Mode::Auto,
            }
        }

        data = match mode {
            Mode::Auto => [RGB8::new(128, 0, 0); 2],
            Mode::Manual => [RGB8::new(0, 0, 128); 2],
        };

        power = match mode {
            Mode::Auto => <BoundedU8<0, 63>>::new(0).unwrap(),
            Mode::Manual => <BoundedU8<0, 63>>::new(63).unwrap(),
        };

        emc.set_fan_power(power).unwrap();

        ws2812.write(&data).await;

        match emc.fan_rpm() {
            Ok(rpm) => defmt::info!("fan rpm {=u16}", rpm),
            Err(e) => defmt::error!("error reading temp internal {:?}", e),
        }

        match emc.temp_internal() {
            Ok(t) => defmt::info!("temp internal {=i8}", t),
            Err(e) => defmt::error!("error reading temp internal {:?}", e),
        }

        match emc.temp_external_precise() {
            Ok(t) => defmt::info!("temp external {=f32}C", t),
            Err(e) => defmt::error!("error reading temp internal {:?}", e),
        }

        ticker.next().await;
    }
}

#[embassy_executor::task]
async fn mode_switcher(mut button: Input<'static>, mode: &'static mut Mode) {
    loop {
        button.wait_for_rising_edge().await;

        *mode = match mode {
            Mode::Auto => Mode::Manual,
            Mode::Manual => Mode::Auto,
            // Mode::Manual => Mode::Off,
            // Mode::Off => Mode::Auto,
        };
    }
}

#[embassy_executor::task]
async fn blade0_listener(uart0: &'static mut BufferedUart) {
    loop {
        let mut buf = [0u8; 3];
        uart0.read_exact(&mut buf).await.unwrap();
    }
}

#[embassy_executor::task]
async fn blade1_listener(uart1: &'static mut BufferedUart) {
    loop {
        let mut buf = [0u8; 3];
        uart1.read_exact(&mut buf).await.unwrap();
    }
}

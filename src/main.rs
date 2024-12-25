#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use core::cell;
use panic_halt as _;

const TIMER_TIME: i32 = 420;

const PRESCALER: u32 = 1024;
const TIMER_COUNTS: u32 = 125;

const MILLIS_INCREMENT: u32 = PRESCALER * TIMER_COUNTS / 16000;

static MILLIS_COUNTER: avr_device::interrupt::Mutex<cell::Cell<u32>> =
    avr_device::interrupt::Mutex::new(cell::Cell::new(0));

fn millis_init(tc0: arduino_hal::pac::TC0) {
    // Configure the timer for the above interval (in CTC mode)
    // and enable its interrupt.
    tc0.tccr0a.write(|w| w.wgm0().ctc());
    tc0.ocr0a.write(|w| w.bits(TIMER_COUNTS as u8));
    tc0.tccr0b.write(|w| match PRESCALER {
        8 => w.cs0().prescale_8(),
        64 => w.cs0().prescale_64(),
        256 => w.cs0().prescale_256(),
        1024 => w.cs0().prescale_1024(),
        _ => panic!(),
    });
    tc0.timsk0.write(|w| w.ocie0a().set_bit());

    // Reset the global millisecond counter
    avr_device::interrupt::free(|cs| {
        MILLIS_COUNTER.borrow(cs).set(0);
    });
}

#[avr_device::interrupt(atmega328p)]
fn TIMER0_COMPA() {
    avr_device::interrupt::free(|cs| {
        let counter_cell = MILLIS_COUNTER.borrow(cs);
        let counter = counter_cell.get();
        counter_cell.set(counter + MILLIS_INCREMENT);
    })
}

fn millis() -> u32 {
    avr_device::interrupt::free(|cs| MILLIS_COUNTER.borrow(cs).get())
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut timer = TIMER_TIME;
    let mut time_start = 0;
    let mut time_now = 0;

    millis_init(dp.TC0);

    let start_button = pins.d5.into_pull_up_input();
    let mut relay = pins.d6.into_output();

    unsafe { avr_device::interrupt::enable() };

    loop {
        let time = millis();
        if time - time_now > 1 {
            time_now = time;
            if start_button.is_low() {
                if time_start <= 0 {
                    time_start = 255;
                    relay.set_high();
                } else {
                    time_start = 0;
                    relay.set_low();
                }
            }
        }

        if time_start >= 0 {
            if timer <= 0 {
                relay.set_low();
                timer = TIMER_TIME;
                time_start = 0;
            }
            timer = timer - 1;
            arduino_hal::delay_ms(1000);
        }
    }
}

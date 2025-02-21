use std::{cmp::min_by_key, thread::sleep, time::Duration};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use hidapi::{HidApi, HidDevice};
use indicatif::{ProgressBar, ProgressStyle};
use tap::Pipe;

// 0x9a63 = 24MD4KL
// 0x9a70 = 27MD5KL
// 0x9a40 = 27MD5KA

const VENDOR_ID: u16 = 0x43e;
const PRODUCT_ID: u16 = 0x9a70;

const MAX_BRIGHTNESS: u16 = 0xd2f0;
const MIN_BRIGHTNESS: u16 = 0x0190;

const STEP: u16 = 540;

/// 1%
const STEPS: [u16; 100] = [
    540, 1080, 1620, 2160, 2700, 3240, 3780, 4320, 4860, 5400, 5940, 6480, 7020, 7560, 8100, 8640,
    9180, 9720, 10260, 10800, 11340, 11880, 12420, 12960, 13500, 14040, 14580, 15120, 15660, 16200,
    16740, 17280, 17820, 18360, 18900, 19440, 19980, 20520, 21060, 21600, 22140, 22680, 23220,
    23760, 24300, 24840, 25380, 25920, 26460, 27000, 27540, 28080, 28620, 29160, 29700, 30240,
    30780, 31320, 31860, 32400, 32940, 33480, 34020, 34560, 35100, 35640, 36180, 36720, 37260,
    37800, 38340, 38880, 39420, 39960, 40500, 41040, 41580, 42120, 42660, 43200, 43740, 44280,
    44820, 45360, 45900, 46440, 46980, 47520, 48060, 48600, 49140, 49680, 50220, 50760, 51300,
    51840, 52380, 52920, 53460, 54000,
];

fn get_brightness(uf: &HidDevice) -> u16 {
    let mut data = [0; 7];

    let _res = uf.get_feature_report(&mut data);

    // println!("res = {:?}", res);

    // println!("data = {:?}", data);

    let x = data[1] as u16;
    let y = u16::from(data[2]) << 8;

    x + y
}

fn find_nearest(x: u16) -> u16 {
    fn calc_diff(a: u16, b: u16) -> u16 {
        if a > b { a - b } else { b - a }
    }

    STEPS
        .iter()
        .fold(0_u16, |acc, e| min_by_key(acc, *e, |y| calc_diff(x, *y)))
}

#[test]
fn test_get_brightness() {
    let hidapi = HidApi::new().unwrap();
    let uf = find_oldest_ultrafine_5k(&hidapi).unwrap();

    let brightness = get_brightness(&uf);

    println!("brightness = {}", brightness);
    println!("nearest    = {}", find_nearest(brightness));
}

fn set_brightness(uf: &HidDevice, val: u16) {
    if !STEPS.contains(&val) {
        return;
    }

    let data: [u8; 7] = [
        0,
        (val & 0x00ff) as u8,
        ((val >> 8) & 0x00ff) as u8,
        0,
        0,
        0,
        0,
    ];

    let _res = uf.send_feature_report(&data);

    // println!("res = {:?}", res);
}

fn find_oldest_ultrafine_5k(hidapi: &HidApi) -> Option<HidDevice> {
    for device in hidapi.device_list() {
        if VENDOR_ID == device.vendor_id() {
            // println!();
            // println!("product id   = {:x}", device.product_id());
            // println!(
            //     "product      = {}",
            //     device.product_string().unwrap_or("N/A")
            // );
            // println!(
            //     "manufacturer = {}",
            //     device.manufacturer_string().unwrap_or("N/A")
            // );

            if device.product_id() == PRODUCT_ID
                && device.product_string().unwrap_or_default() == "HID BRIGHTNESS"
            {
                return Some(device.open_device(hidapi).unwrap());
            }
        }
    }

    None
}

fn brightness_percent(brightness: u16) -> usize {
    STEPS.iter().position(|step| *step == brightness).unwrap() + 1
}

fn main() {
    // println!("Hello, world!");

    let hidapi = HidApi::new().unwrap();

    // println!();

    let Some(uf) = find_oldest_ultrafine_5k(&hidapi) else {
        println!("hasn't oldest ultrafine 5k display");
        return;
    };

    // println!();

    enable_raw_mode().unwrap();

    let progress = ProgressBar::new(100);

    progress.set_style(ProgressStyle::with_template("brightness = {pos}% ({msg})").unwrap());

    {
        let brightness = get_brightness(&uf).pipe(find_nearest);
        progress.set_position(brightness_percent(brightness) as u64);
        progress.set_message(brightness.to_string());
    }

    loop {
        let event = event::read().unwrap();

        let Event::Key(key) = event else {
            continue;
        };

        if key.kind != KeyEventKind::Release {
            continue;
        }

        let mut brightness = get_brightness(&uf).pipe(find_nearest);

        match key.code {
            KeyCode::Left => {
                if STEP >= MIN_BRIGHTNESS + brightness {
                    brightness = MIN_BRIGHTNESS;
                } else {
                    brightness -= STEP;
                }
            }

            KeyCode::Down => {
                if STEP * 5 >= MIN_BRIGHTNESS + brightness {
                    brightness = MIN_BRIGHTNESS;
                } else {
                    brightness -= STEP * 5;
                }
            }

            KeyCode::Right => {
                if brightness + STEP >= MAX_BRIGHTNESS {
                    brightness = MAX_BRIGHTNESS;
                } else {
                    brightness += STEP;
                }
            }

            KeyCode::Up => {
                if brightness + STEP * 5 >= MAX_BRIGHTNESS {
                    brightness = MAX_BRIGHTNESS;
                } else {
                    brightness += STEP * 5;
                }
            }

            KeyCode::Char('q') => {
                break;
            }

            _ => {
                continue;
            }
        }

        set_brightness(&uf, brightness);

        progress.set_position(brightness_percent(brightness) as u64);

        progress.set_message(brightness.to_string());

        sleep(Duration::from_millis(50));
    }

    disable_raw_mode().unwrap();
}

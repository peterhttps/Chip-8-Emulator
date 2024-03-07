use std::env;
use std::time::{Duration, Instant};
use std::fs::File;
use std::io::Read;
use chip8_core::*;
use sdl2::event::Event;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::keyboard::Keycode;
use sdl2::audio::{AudioSpecDesired, AudioDevice, AudioCallback};

const TICKS_PER_FRAME: usize = 10;
const SCALE: u32 = 15;
const WINDOW_WIDTH: u32 = (SCREEN_WIDTH as u32) * SCALE;
const WINDOW_HEIGHT: u32 = (SCREEN_HEIGHT as u32) * SCALE;
const FRAME_DURATION: Duration = Duration::from_millis(1000 / 60);

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: cargo run path/to/game");
        return;
    }

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem.window("Chip8 Emulator", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .opengl()
        .build()
        .unwrap();
    
    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    canvas.clear();
    canvas.present();

    let audio_subsystem = sdl_context.audio().unwrap();
    let desired_spec = AudioSpecDesired {
        freq: Some(44100),
        channels: Some(1),
        samples: None,
    };

    let beep_frequency = 440.0;
    let volume = 0.10;

    let device: AudioDevice<SquareWave> = audio_subsystem.open_playback(None, &desired_spec, |spec| {
        // Calculate the phase increment for generating the square wave
        SquareWave {
            phase_inc: beep_frequency * 2.0 * std::f32::consts::PI / spec.freq as f32,
            phase: 0.0,
            volume,
        }
    }).unwrap();

    let sound_closure = || {
        device.resume();
        std::thread::sleep(Duration::from_millis(50));
        device.pause();
    };

    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut chip8 = Emu::new();
    let mut rom = File::open(&args[1]).expect("Unable to open the file");
    let mut buffer = Vec::new();
    rom.read_to_end(&mut buffer).expect("Unable to read the file");
    chip8.load(&buffer);

    let mut last_tick = Instant::now();

    'gameloop: loop {
        let now = Instant::now();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'gameloop;
                }
                Event::KeyDown { keycode: Some(key), .. } => {
                    if let Some(btn) = key2btn(key) {
                        chip8.keypress(btn, true);
                    }
                }
                Event::KeyUp { keycode: Some(key), .. } => {
                    if let Some(btn) = key2btn(key) {
                        chip8.keypress(btn, false);
                    }
                }
                _ => {}
            }
        }

        if now.duration_since(last_tick) >= FRAME_DURATION {
            for _ in 0..TICKS_PER_FRAME {
                chip8.tick();
            }
            chip8.tick_timers(sound_closure);

            last_tick = now;
        }

        draw_screen(&chip8, &mut canvas);

        let frame_time = now.elapsed();
        if frame_time < FRAME_DURATION {
            std::thread::sleep(FRAME_DURATION - frame_time);
        }
    }
}

fn draw_screen(emu: &Emu, canvas: &mut Canvas<Window>) {
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();

    let screen_buf = emu.get_display();

    canvas.set_draw_color(Color::RGB(255, 255, 255));
    for (i, pixel) in screen_buf.iter().enumerate() {
        if *pixel {
            let x = (i % SCREEN_WIDTH) as i32;
            let y = (i / SCREEN_WIDTH) as i32;

            let rect = Rect::new((x * SCALE as i32) as i32, (y * SCALE as i32) as i32, SCALE, SCALE);
            canvas.fill_rect(rect).unwrap()
        }
    }

    canvas.present();
}

fn key2btn(key: Keycode) -> Option<usize> {
    match key {
        Keycode::Num1 => Some(0x1),
        Keycode::Num2 => Some(0x2),
        Keycode::Num3 => Some(0x3),
        Keycode::Num4 => Some(0xC),
        Keycode::Q => Some(0x4),
        Keycode::W => Some(0x5),
        Keycode::E => Some(0x6),
        Keycode::R => Some(0xD),
        Keycode::A => Some(0x7),
        Keycode::S => Some(0x8),
        Keycode::D => Some(0x9),
        Keycode::F => Some(0xE),
        Keycode::Z => Some(0xA),
        Keycode::X => Some(0x0),
        Keycode::C => Some(0xB),
        Keycode::V => Some(0xF),
        _ => None,
    }
}

struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32,
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        // Generate a square wave
        for x in out.iter_mut() {
            *x = if self.phase.sin() >= 0.0 { self.volume } else { -self.volume };
            self.phase = (self.phase + self.phase_inc) % std::f32::consts::PI * 2.0;
        }
    }
}

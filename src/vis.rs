use std::{path, thread};
use std::sync::mpsc;
use model::{Game, Action, Player, World, Vehicle, VehicleType};
use super::my_strategy::side::Side;
use super::my_strategy::geom::Rect;
use super::my_strategy::formation::Formations;

const CONSOLE_HEIGHT: u32 = 32;
const BORDER_WIDTH: u32 = 16;
const SCREEN_WIDTH: u32 = 800;
const SCREEN_HEIGHT: u32 = 600;

use piston_window::{
    OpenGL,
    PistonWindow,
    WindowSettings,
    TextureSettings,
    Viewport,
    Glyphs,
    EventLoop,
    PressEvent,
    Button,
    Key
};

pub struct Visualizer {
    tx: mpsc::Sender<DrawPacket>,
    rx: mpsc::Receiver<Trigger>,
    pause_tick: i32,
}

impl Visualizer {
    pub fn bootstrap<F>(runner_proc: F) where F: FnOnce(&mut Visualizer) -> ::std::io::Result<()> + Send + Sync + 'static {
        let (master_tx, slave_rx) = mpsc::channel();
        let (slave_tx, master_rx) = mpsc::channel();
        let _slave = thread::Builder::new()
            .name("strategy master".to_string())
            .spawn(move || {
                let mut visualizer = Visualizer {
                    tx: master_tx,
                    rx: master_rx,
                    pause_tick: 0,
                };
                runner_proc(&mut visualizer).unwrap();
            })
            .unwrap();

        painter_loop(&slave_tx, &slave_rx);
    }

    pub fn tick(&mut self, _me: &Player, world: &World, _game: &Game, _action: &Action, allies: &mut Formations, enemies: &mut Formations) {
        let mut draw = Vec::new();
        self.draw_vehicles(allies, &mut draw);
        self.draw_vehicles(enemies, &mut draw);
        self.tx.send(DrawPacket {
            tick_index: world.tick_index,
            world_width: world.width,
            world_height: world.height,
            elements: draw,
        }).unwrap();

        loop {
            match self.rx.recv().unwrap() {
                Trigger::PaintingDone if world.tick_index < self.pause_tick =>
                    break,
                Trigger::PaintingDone =>
                    (),
                Trigger::PauseAfter1 | Trigger::PauseAfter10 if world.tick_index < self.pause_tick =>
                    (),
                Trigger::PauseAfter1 => {
                    self.pause_tick += 1;
                    break;
                },
                Trigger::PauseAfter10 => {
                    self.pause_tick += 10;
                    break;
                },
            }
        }
    }

    fn draw_vehicles(&self, formations: &mut Formations, draw: &mut Vec<Draw>) {
        let side = formations.side;
        let mut forms_iter = formations.iter();
        while let Some(mut form) = forms_iter.next() {
            draw.push(Draw::Formation {
                side,
                kind: form.kind().clone(),
                bbox: form.bounding_box().clone(),
            });
            for vehicle in form.vehicles() {
                draw.push(Draw::Vehicle {
                    side,
                    vehicle: vehicle.clone(),
                });
            }
        }
    }
}

fn painter_loop(tx: &mpsc::Sender<Trigger>, rx: &mpsc::Receiver<DrawPacket>) {
    let opengl = OpenGL::V4_1;
    let mut window: PistonWindow = WindowSettings::new("aicup", [SCREEN_WIDTH, SCREEN_HEIGHT])
        .exit_on_esc(true)
        .opengl(opengl)
        .build()
        .unwrap();
    window.events.set_max_fps(16);
    window.events.set_ups(20);
    window.events.set_ups_reset(0);
    // window.events.set_lazy(true);
    println!("events: {:?}", window.events.get_event_settings());

    let mut font_path = path::PathBuf::from("assets");
    font_path.push("FiraSans-Regular.ttf");
    let mut glyphs = Glyphs::new(&font_path, window.factory.clone(), TextureSettings::new()).unwrap();

    let mut draw_packet = rx.recv().unwrap();
    let mut draw_confirmed = false;
    while let Some(event) = window.next() {
        window.draw_2d(&event, |context, g2d| {
            use piston_window::{clear, text, rectangle, Transformed};
            clear([0.0, 0.0, 0.0, 1.0], g2d);
            text::Text::new_color([0.0, 1.0, 0.0, 1.0], 16).draw(
                &format!("{} |", draw_packet.tick_index),
                &mut glyphs,
                &context.draw_state,
                context.transform.trans(5.0, 20.0),
                g2d
            ).unwrap();
            if let Some(tr) = ViewportTranslator::new(&context.viewport, draw_packet.world_width, draw_packet.world_height) {
                for element in draw_packet.elements.iter() {
                    match element {
                        &Draw::Vehicle { side, ref vehicle, } => {
                            let color = vehicle_color(side, vehicle.kind);
                            let coords = [
                                tr.x(vehicle.x) - tr.scale_x(vehicle.radius),
                                tr.y(vehicle.y) - tr.scale_y(vehicle.radius),
                                tr.scale_x(vehicle.radius) * 2.,
                                tr.scale_y(vehicle.radius) * 2.,
                            ];
                            rectangle(color, coords, context.transform, g2d);
                        },
                        &Draw::Formation { side, kind, ref bbox, } => {
                            let mut color = vehicle_color(side, kind);
                            color[3] = 0.1;
                            let coords = [
                                tr.x(bbox.left),
                                tr.y(bbox.top),
                                tr.scale_x(bbox.right - bbox.left),
                                tr.scale_y(bbox.bottom - bbox.top),
                            ];
                            rectangle(color, coords, context.transform, g2d);
                        },
                    }
                }
            }
            if !draw_confirmed {
                tx.send(Trigger::PaintingDone).unwrap();
                draw_confirmed = true;
            }
        });

        match event.press_args() {
            Some(Button::Keyboard(Key::N)) =>
                tx.send(Trigger::PauseAfter1).unwrap(),
            Some(Button::Keyboard(Key::M)) =>
                tx.send(Trigger::PauseAfter10).unwrap(),
            _ =>
                (),
        }

        match rx.try_recv() {
            Err(mpsc::TryRecvError::Empty) =>
                (),
            Err(mpsc::TryRecvError::Disconnected) =>
                break,
            Ok(packet) => {
                draw_packet = packet;
                draw_confirmed = false;
            },
        };
    }
}

struct ViewportTranslator {
    scale_x: f64,
    scale_y: f64,
}

impl ViewportTranslator {
    fn new(viewport: &Option<Viewport>, world_width: f64, world_height: f64) -> Option<ViewportTranslator> {
        let (w, h) = viewport
            .map(|v| (v.draw_size[0], v.draw_size[1]))
            .unwrap_or((SCREEN_WIDTH, SCREEN_HEIGHT));

        if (w <= 2 * BORDER_WIDTH) || (h <= BORDER_WIDTH + CONSOLE_HEIGHT) {
            None
        } else {
            Some(ViewportTranslator {
                scale_x: (w - BORDER_WIDTH - BORDER_WIDTH) as f64 / world_width,
                scale_y: (h - BORDER_WIDTH - CONSOLE_HEIGHT) as f64 / world_height,
            })
        }
    }

    fn scale_x(&self, x: f64) -> f64 {
        x * self.scale_x
    }

    fn scale_y(&self, y: f64) -> f64 {
        y * self.scale_y
    }

    fn x(&self, x: f64) -> f64 {
        self.scale_x(x) + BORDER_WIDTH as f64
    }

    fn y(&self, y: f64) -> f64 {
        self.scale_y(y) + CONSOLE_HEIGHT as f64
    }
}

enum Trigger {
    PaintingDone,
    PauseAfter1,
    PauseAfter10,
}

struct DrawPacket {
    tick_index: i32,
    world_width: f64,
    world_height: f64,
    elements: Vec<Draw>,
}

enum Draw {
    Vehicle { side: Side, vehicle: Vehicle, },
    Formation { side: Side, kind: Option<VehicleType>, bbox: Rect, },
}

fn vehicle_color(side: Side, kind: Option<VehicleType>) -> [f32; 4] {
    let rgb: [u8; 3] = match (side, kind) {
        (Side::Allies, Some(VehicleType::Arrv)) =>
            [165, 42, 42], // brown
        (Side::Allies, Some(VehicleType::Fighter)) =>
            [255, 192, 203], // pink
        (Side::Allies, Some(VehicleType::Helicopter)) =>
            [255, 255, 0], // yellow
        (Side::Allies, Some(VehicleType::Ifv)) =>
            [255, 165, 0], // orange
        (Side::Allies, Some(VehicleType::Tank)) =>
            [255, 0, 0], // red
        (Side::Enemies, Some(VehicleType::Arrv)) =>
            [0, 255, 0], // grey
        (Side::Enemies, Some(VehicleType::Fighter)) =>
            [128, 0, 128], // purple
        (Side::Enemies, Some(VehicleType::Helicopter)) =>
            [128, 128, 128], // green
        (Side::Enemies, Some(VehicleType::Ifv)) =>
            [0, 128, 128], // teal
        (Side::Enemies, Some(VehicleType::Tank)) =>
            [0, 0, 255], // blue
        (.., None) =>
            [255, 255, 255], // white
    };
    [rgb[0] as f32 / 255., rgb[1] as f32 / 255., rgb[2] as f32 / 255., 1.0]
}

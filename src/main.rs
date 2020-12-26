use sdl2::event::Event;
use sdl2::gfx::primitives::DrawRenderer;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use std::ops::Add;
use std::sync::mpsc;
use std::sync::mpsc::Sender;

#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone)]
struct Coord(i8, i8);

impl Add<Coord> for Coord {
    type Output = Coord;

    fn add(self, rhs: Coord) -> Self::Output {
        Coord(self.0 + rhs.0, self.1 + rhs.1)
    }
}

impl Add<&Coord> for Coord {
    type Output = Coord;

    fn add(self, rhs: &Coord) -> Self::Output {
        Coord(self.0 + rhs.0, self.1 + rhs.1)
    }
}

impl std::ops::SubAssign for Coord {
    fn sub_assign(&mut self, rhs: Self) {
        *self = Self(self.0 - rhs.0, self.1 - rhs.1)
    }
}

impl std::ops::AddAssign for Coord {
    fn add_assign(&mut self, rhs: Coord) {
        *self = Coord(self.0 + rhs.0, self.1 + rhs.1)
    }
}

#[derive(Debug)]
struct Board {
    moves_made: Vec<Coord>,
    current: Coord,
    moves_to_make: Vec<Vec<Coord>>,
    board: [i8; 64],
    moves: [Coord; 8],
}

#[derive(Debug)]
enum Mutation {
    Move,
    Rollback,
    Stop,
}

impl Board {
    pub fn value_at(&self, coord: Coord) -> i8 {
        self.board[Board::index_of(coord)]
    }

    fn index_of(coord: Coord) -> usize {
        (coord.0 * 8 + coord.1) as usize
    }

    pub fn set_value_at(&mut self, coord: Coord, val: i8) {
        self.board[Board::index_of(coord)] = val
    }

    pub fn new() -> Board {
        let mut ret = Board {
            moves_made: Vec::new(),
            current: Coord(0, 0),
            moves_to_make: Vec::new(),
            board: [0; 64],
            moves: {
                let combs = [1i8, 2, -1, -2];
                let mut ret = [Coord(0, 0); 8];
                combs
                    .iter()
                    .flat_map(|i| combs.iter().map(move |j| Coord(*i, *j)))
                    .filter(|c| c.0.abs() != c.1.abs())
                    .enumerate()
                    .for_each(|(i, x)| {
                        ret[i] = x;
                    });
                ret
            },
        };
        ret.moves_to_make.push(ret.available_moves());
        ret
    }

    pub fn is_on_board(c: Coord) -> bool {
        c.0 >= 0 && c.0 < 8 && c.1 >= 0 && c.1 < 8
    }

    pub fn can_move(&self, c: Coord) -> bool {
        self.value_at(c) == 0i8
    }

    pub fn available_moves(&self) -> Vec<Coord> {
        self.moves
            .iter()
            .copied()
            .filter(|m| {
                let c = self.current + m;
                let ret = Board::is_on_board(c) && self.can_move(c);
                ret
            })
            .collect()
    }

    pub fn make_move(&mut self, c: Coord) {
        self.current += c;
        self.moves_made.push(c);
        self.set_value_at(self.current, self.moves_made.len() as i8);
    }

    pub fn rollback(&mut self) {
        self.set_value_at(self.current, 0);
        let rb = self.moves_made.pop().expect("Logic error");
        self.current -= rb;
    }

    pub fn apply_best_move(&mut self) {
        //println!("apply board is {:?}", self);
        //val am = self.available_moves(self);
        let mut best: Option<(Coord, usize, usize)> = None;
        for (i, available_move) in self
            .moves_to_make
            .last()
            .unwrap()
            .clone()
            .iter()
            .enumerate()
        {
            self.make_move(*available_move);
            let am = self.available_moves();
            let new_len = am.len();
            self.rollback();
            best = match best {
                None => Some((*available_move, new_len, i)), // First loop
                Some((_, best_len, _)) if new_len < best_len => Some((*available_move, new_len, i)), // New best
                _ => best, // Not a new best - leave as is
            }
        }
        assert!(best.is_some());
        let (c, _, idx) = best.unwrap();
        self.make_move(c);
        self.moves_to_make.last_mut().unwrap().remove(idx);
        self.moves_to_make.push(self.available_moves());
    }

    pub fn get_action(&self) -> Mutation {
        use Mutation::*;
        match self.moves_to_make.last() {
            Some(v) if v.is_empty() => Rollback,
            Some(_) => Move,
            None => Stop,
        }
    }

    pub fn is_closed_tour(&self) -> bool {
        return self
            .moves
            .iter()
            .any(|m| self.current + m == *(self.moves_made.first().unwrap()));
    }

    pub fn do_loop(&mut self, sender: Sender<Vec<Coord>>) {
        loop {
            let m = self.get_action();
            match m {
                Mutation::Move => {
                    self.apply_best_move();
                    if self.moves_made.len() == 64 && self.is_closed_tour() {
                        sender.send(self.moves_made.clone()).unwrap();
                    }
                }
                Mutation::Rollback => {
                    self.rollback();
                    self.moves_to_make.pop();
                }
                Mutation::Stop => {
                    break;
                }
            }
        }
    }
}

fn main() -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let ev = sdl_context.event().unwrap();
    let video_subsystem = sdl_context.video()?;
    let window = video_subsystem
        .window("A Knights Tour", 960, 960)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;
    let mut canvas = window
        .into_canvas()
        .software()
        .build()
        .map_err(|e| e.to_string())?;
    let event_type = unsafe { ev.register_event().unwrap() };
    let (tx, rx) = mpsc::channel();
    let mut b = Board::new();

    std::thread::spawn(move || {
        b.do_loop(tx);
    });

    let mut current_vec: Option<Vec<Coord>> = None;
    'mainloop: loop {
        if let Ok(vec) = rx.try_recv() {
            current_vec = Some(vec);
            ev.push_event(sdl2::event::Event::User {
                timestamp: 0,
                window_id: 0,
                type_: event_type,
                code: event_type as i32,
                data1: std::ptr::null_mut::<libc::c_void>(),
                data2: std::ptr::null_mut::<libc::c_void>(),
            })?
        }

        for event in sdl_context.event_pump()?.poll_iter() {
            match event {
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                }
                | Event::Quit { .. } => break 'mainloop,
                _ => {}
            }
        }

        canvas.set_draw_color(Color::RGBA(0, 0, 0, 255));
        canvas.clear();
        const SZ: i32 = 120;
        canvas.set_draw_color(Color::RGBA(255, 255, 255, 255));
        for x in 0i32..8 {
            for y in 0i32..8 {
                if (x + y) % 2 == 0 {
                    canvas.fill_rect(Rect::new(x * SZ, y * SZ, SZ as u32, SZ as u32))?
                }
            }
        }

        // const CIRCLE_RADIUS: i16 = 40; //i16;
        let red = Color::RGBA(255, 0, 0, 255);
        // let green = Color::RGBA(0, 255, 0, 255);
        // let blue = Color::RGBA(0, 0, 255, 255);
        if let Some(xs) = &current_vec {
            let mut current = Coord(0, 0);
            let mut last: Option<Point> = None;
            let mut first: Option<Point> = None;
            for &x in xs.iter() {
                current += x;
                let c = &current;
                let new = Point::new(
                    (c.0 as i32 * SZ + SZ / 2) as i32,
                    (c.1 as i32 * SZ + SZ / 2) as i32,
                );

                if first.is_none() {
                    first = Some(new)
                }

                if let Some(l) = last {
                    canvas
                        .thick_line(l.x as i16, l.y as i16, new.x as i16, new.y as i16, 12, red)
                        .unwrap()
                };
                // canvas
                // .filled_circle(new.x as i16, new.y as i16, CIRCLE_RADIUS, green)
                // .unwrap(),
                last = Some(new)
            }

            /*            if let Some(last_point) = last {
                canvas
                    .filled_circle(
                        last_point.x as i16,
                        last_point.y as i16,
                        CIRCLE_RADIUS,
                        blue,
                    )
                    .unwrap();
            } */
            if let (Some(f), Some(l)) = (first, last) {
                canvas
                    .thick_line(f.x as i16, f.y as i16, l.x as i16, l.y as i16, 12, red)
                    .unwrap()
            }
        }
        canvas.present();
    }
    Ok(())
}

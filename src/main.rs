use std::ops::Add;
use sdl2::pixels::{Color};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::rect::{Rect, Point};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use sdl2::gfx::primitives::DrawRenderer;

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
    fn add_assign(&mut self, rhs: Self) {
        *self = Self(self.0 + rhs.0, self.1 + rhs.1)
    }
}

#[derive(Debug)]
struct Board {
    moves_made: Vec<Coord>,
    current: Coord,
    moves_to_make: Vec<Vec<Coord>>,
    board: [i8; 64],
    moves : [Coord;8]
}

impl Board {
    pub fn value_at(&self, coord: Coord) -> i8 {
        self.board[Board::index_of(coord)]
    }

    fn index_of(coord : Coord) ->  usize {
        (coord.0 * 8 + coord.1) as usize
    }

    pub fn set_value_at(&mut self, coord: Coord, val: i8) {
        self.board[Board::index_of(coord)] = val
    }

    pub fn new() -> Board {
        let mut ret = Board {
            moves_made: Vec::new(),
            current: Coord(0,0),
            moves_to_make: Vec::new(),
            board: [0; 64],
            moves : {
                let combs = [1i8, 2, -1, -2];
                let mut ret = [Coord(0,0);8];
                combs.iter().flat_map(|i| {
                    combs.iter().map(move |j| {Coord(*i, *j) } ) })
                    .filter(|c| c.0.abs() != c.1.abs())
                    .enumerate()
                    .for_each( |(i,x)| {
                        ret[i] = x;
                    } );
                ret
            }
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
        self.moves.iter().copied().filter(|m| {
            let c = self.current + m;
            let ret = Board::is_on_board(c) && self.can_move(c);
            ret
        }).collect()
    }

    pub fn do_loop(&mut self,
                   sender: Sender<Vec<Coord>>,
    ) -> () {
        loop {
            match self.moves_to_make.last_mut() {
                Some(v) => {
                    if v.is_empty() {
                        self.moves_to_make.pop();
                        self.set_value_at(self.current, 0);
                        let rollback = self.moves_made.pop().expect("Logic error");
                        self.current -= rollback;
                    } else {
                        let to_move = v.pop().expect("Logic error");
                        self.current += to_move;
                        self.moves_made.push(to_move);
                        self.set_value_at(self.current, self.moves_made.len() as i8);
                        self.moves_to_make.push(self.available_moves());
                        if self.moves_made.len() == 64 {
                            sender.send(self.moves_made.clone()).expect("Failed to send");
                        }
                    }
                }
                None => {
                    println!("Game over");
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
    let mut canvas = window.into_canvas().software().build().map_err(|e| e.to_string())?;
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
                data1: 0 as *mut libc::c_void,
                data2: 0 as *mut libc::c_void,
            })?
        }

        for event in sdl_context.event_pump()?.poll_iter() {
            match event {
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } |
                Event::Quit { .. } => break 'mainloop,
                _ => {}
            }
        }

        canvas.set_draw_color(Color::RGBA(0, 0, 0, 255));
        canvas.clear();
        let sz = 120i32;
        canvas.set_draw_color(Color::RGBA(255, 255, 255, 255));
        for x in 0i32..8 {
            for y in 0i32..8 {
                if (x + y) % 2 == 0 {
                    canvas.fill_rect(Rect::new(x * sz, y * sz, sz as u32, sz as u32))?
                }
            }
        }
        let red = Color::RGBA(255, 0, 0, 255);
        if let Some(xs) = &current_vec {
            let mut current = Coord(0,0);
            let mut last: Option<Point> = None;
            for &x in xs.iter() {
                current += x;
                let c = &current;
                let new = Point::new((c.0 as i32 * sz + sz / 2) as i32, (c.1 as i32 * sz + sz / 2) as i32);
                if let Some(l) = last {
                    canvas.thick_line(l.x as i16, l.y as i16, new.x as i16, new.y as i16, 12, red)?
                }
                last = Some(new)
            }
        }

        canvas.present();
    }
    return Ok(());
}

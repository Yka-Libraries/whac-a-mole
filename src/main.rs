use crossterm::{
    cursor::{Hide, MoveTo, MoveToNextLine},
    event::{read, Event, KeyCode},
    execute,
    style::style,
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen, SetTitle,
    },
    Result,
};
use drawille::{Canvas};
use lazy_static::lazy_static;
use rand::Rng;
use std::{
    io::stdout,
    process::exit,
    sync::{Arc, Mutex},
    thread::{self},
    time::Duration,
};

lazy_static! {
    static ref GAME: Arc<Mutex<Game>> = Arc::new(Mutex::new(Game::new(&Dimension {
        width: 70,
        height: 25,
    })));
}

// https://unicode-table.com/cn/blocks/box-drawing/
const CHAR_VIEW_LIST: [char; 16] = [
    ' ', '上', '下', '║', '左', '╚', '╔', '╠', '右', '╝', '╗', '╣', '═', '╩', '╦', '╬',
];

type Matrix<T> = Vec<Vec<T>>;

fn clear_terminal() {
    let _ = execute!(stdout(), MoveTo(0, 0));
}

fn go_alternate_screen() {
    let _ = execute!(stdout(), EnterAlternateScreen, Hide);
}

fn leave_alternate_screen() {
    let _ = execute!(stdout(), LeaveAlternateScreen);
}

fn get_random_num(from: usize, to: usize) -> usize {
    return rand::thread_rng().gen_range(from..=to);
}

fn write_words(views: &mut Matrix<char>, left: usize, top: usize, words: String) {
    for (i, ch) in words.chars().enumerate() {
        // 数组是横着放的
        views[top][left + i] = ch;
    }
}

#[derive(Clone, Debug)]
struct Dimension {
    width: usize,
    height: usize,
}

#[derive(Clone, Debug)]
struct Hole {
    x: usize,
    y: usize,
}

#[derive(Clone, Debug)]
struct GameView {
    points: Matrix<usize>,
    views: Matrix<char>,
    hole_points: Vec<Hole>,
    hole_marmots: Vec<Marmot>,
    size: Dimension,
}

impl GameView {
    fn new(size: &Dimension) -> Self {
        GameView {
            points: vec![vec![0; size.width]; size.height],
            views: vec![vec![' '; size.width]; size.height],
            hole_points: vec![],
            hole_marmots: vec![],
            size: size.clone(),
        }
    }

    // 以左上角原点为基础构建游戏框架
    fn build_block(&mut self, top: usize, bottom: usize, left: usize, right: usize) {
        let Dimension { width, height } = self.size;
        if top >= height
            || bottom >= height
            || left >= width
            || right >= width
            || left == right
            || top == bottom
        {
            eprintln!("\nCan not build the block! The parameters is incorrect!\nTraceBack:\n\tleft:{} right:{} width_limit:{}\n\ttop:{} bottom:{} height_limit:{}\n", left, right, width, top, bottom, height);
            exit(0);
        }

        for i in (top + 1)..bottom {
            self.points[i][left] |= 3;
            self.points[i][right] |= 3;
        }

        for i in (left + 1)..right {
            self.points[top][i] |= 12;
            self.points[bottom][i] |= 12;
        }

        self.points[top][left] |= 6;
        self.points[top][right] |= 10;
        self.points[bottom][left] |= 5;
        self.points[bottom][right] |= 9;

        self.views = self.update_block_char();
    }

    fn set_hole_points(&mut self, point: Hole) {
        self.hole_points.push(point);
    }

    fn set_hole_marmots(&mut self, marmot: Marmot) {
        self.hole_marmots.push(marmot);
    }

    fn update_block_char(&self) -> Matrix<char> {
        let mut char_vec = vec![];
        for point in &self.points {
            char_vec.push(
                point
                    .iter()
                    .map(|&x| CHAR_VIEW_LIST[x])
                    .collect::<Vec<char>>(),
            );
        }
        char_vec
    }

    fn draw(&self) {
        clear_terminal();
        let mut styled_char_matrix = vec![];
        for lines in &self.views {
            let mut row = vec![];
            for ch in lines {
                row.push(style(ch))
            }
            styled_char_matrix.push(row);
        }

        for row in &styled_char_matrix {
            for &ch in row {
                print!("{}", ch);
            }
            let _ = execute!(stdout(), MoveToNextLine(1));
        }
    }
}

#[derive(Clone, Debug)]
struct Marmot {
    view: String,
    appeared: bool, // 是否出现
}

impl Marmot {
    fn new() -> Self {
        Marmot {
            view: String::from("🐭"),
            appeared: false,
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
enum GameState {
    Stopped,
    Playing,
}

#[derive(Debug)]
struct Game {
    view: GameView,
    state: GameState,
    scores: u128,
    time: u8,
}

impl Game {
    fn new(size: &Dimension) -> Self {
        Game {
            view: GameView::new(size),
            state: GameState::Stopped,
            scores: 0,
            time: 60,
        }
    }
}

fn main() -> Result<()> {
    if let Err(_) = enable_raw_mode() {
        eprintln!("Your terminal does not support raw mode!");
        exit(0);
    }
    go_alternate_screen();

    {
        let _ = execute!(stdout(), SetTitle("打地鼠"));
        let size = Dimension {
            width: 70,
            height: 25,
        };
        let mut game = GAME.lock().unwrap();
        game.view.build_block(0, size.height - 1, 0, size.width - 1);
        game.view.build_block(0, size.height - 1, 0, 40);

        let initial_top = 3;
        let initial_bottom = 7;
        let initial_left = 3;
        let initial_right = 11;
        let horizontal_increment = 12;
        let vertical_increment = 6;
        for i in 0..=8 {
            let horizontal_vector = i % 3;
            let vertical_vector = i / 3;
            let top = initial_top + vertical_increment * vertical_vector;
            let bottom = initial_bottom + vertical_increment * vertical_vector;
            let left = initial_left + horizontal_increment * horizontal_vector;
            let right = initial_right + horizontal_increment * horizontal_vector;
            game.view.build_block(top, bottom, left, right);
            game.view.set_hole_points(Hole {
                x: (left + right) / 2,
                y: (top + bottom) / 2,
            });
            game.view.set_hole_marmots(Marmot::new());
        }
        game.state = GameState::Playing;
    }

    fn start() {
        {
            let mut game = GAME.lock().unwrap();
            let scores = game.scores;
            let time = game.time;
            write_words(&mut game.view.views, 50, 9, format!("Scores: {}", scores));
            write_words(&mut game.view.views, 50, 11, format!("Time: {}", time));
            write_words(&mut game.view.views, 50, 13, format!("q: {}", "quit the game"));
            game.view.draw();
        }

        thread::spawn(|| loop {
            let mut game = GAME.lock().unwrap();
            if game.state == GameState::Stopped {
                return;
            }
            std::thread::sleep(Duration::from_millis(1000));
            let random_num = get_random_num(1, 6);
            let mut marmots = game.view.hole_marmots.clone();
            for idx in 0..9 {
                marmots[idx].appeared = false;
            }
            for idx in 0..9 {
                let points = &game.view.hole_points;
                let point_x = points[idx].x;
                let point_y = points[idx].y;
                drop(points);
                write_words(
                    &mut game.view.views,
                    point_x,
                    point_y,
                    String::from(" "),
                );
            }
            for _ in 0..random_num {
                let random_idx = get_random_num(0, 8);
                let points = &game.view.hole_points;
                let point_x = points[random_idx].x;
                let point_y = points[random_idx].y;
                drop(points);
                marmots[random_idx].appeared = true;
                write_words(
                    &mut game.view.views,
                    point_x,
                    point_y,
                    String::from("🐭"),
                );
            }

            game.view.hole_marmots = marmots;

            let scores = game.scores;
            write_words(&mut game.view.views, 50, 9, format!("Scores: {}", scores));

            game.view.draw();
        });

        thread::spawn(|| loop {
            std::thread::sleep(Duration::from_millis(1000));
            let mut game = GAME.lock().unwrap();

            if game.state == GameState::Stopped {
                return;
            }

            if game.time > 0 {
                game.time -= 1;
            } else {
                game.state = GameState::Stopped;
                write_words(&mut game.view.views, 50, 5, format!("Game is Over!"));
            }

            let time = game.time;
            write_words(&mut game.view.views, 50, 11, format!("Time: {}", time));

            game.view.draw();
        });
    }

    clear_terminal();
    start();
    let mut has_egg = false;
    loop {
        let event = read()?;
        if let Event::Key(key_event) = event {
            match key_event.code {
                KeyCode::Char(ch) => match ch {
                    'q' => break,
                    '1'..='9'  => {
                        let mut game = GAME.lock().unwrap();
                        let marmots = &game.view.hole_marmots;
                        let points = &game.view.hole_points;
                        let idx = ch.to_digit(10).unwrap() as usize - 1;
                        let point_x = points[idx].x;
                        let point_y = points[idx].y;
                        drop(points);
                        if marmots[idx].appeared {
                            write_words(
                                &mut game.view.views,
                                point_x,
                                point_y,
                                String::from("❌"),
                            );
                            game.view.draw();
                            game.scores += 10;
                        }
                    }
                    _ => (),
                },
                _ => {}
            }
        }

        {
            let mut game = GAME.lock().unwrap();
            if game.scores > 1024 && !has_egg {
                game.state = GameState::Stopped;
                has_egg = true;
                clear_terminal();
                let _ = execute!(stdout(), Clear(ClearType::All));
                let mut canvas = Canvas::new(30, 20);
                canvas.text(
                    35,
                    20,
                    150,
                    "1024 cheers! 恭喜你过关啦！！🌈 可以找 chongbayang 拿红包哦~",
                );
                println!("{}", canvas.frame());
            }
        }
    }
    leave_alternate_screen();
    disable_raw_mode()?;

    Ok(())
}

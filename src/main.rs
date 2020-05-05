#![warn(clippy::all)]
#![allow(clippy::mutex_atomic)]
extern crate pretty_env_logger;
#[macro_use]
extern crate log;

mod client;
mod db;
// mod index;
mod model;
// mod print;
mod worker;

use std::fs::File;
use simplelog;
use client::{Client, ClientOptions, Mode};
use db::DB;
use model::*;
use quick_error::quick_error;
use std::error::Error;
use std::thread;
use structopt::StructOpt;
use tuikit::canvas;
use tuikit::prelude::*;
use std::fmt;
use std::str::FromStr;
use std::net::ToSocketAddrs;
use std::fmt::{Display,Formatter};
use std::sync::{
    Arc, Mutex,
};

struct Config {
    sonic_addr: String,
    mongo_addr: String,
}

quick_error! {
    #[derive(Debug)]
    pub enum RuntimeError {
        Unexpected(descr: String) {
            display("Error {}", descr)
        }
    }
}

#[derive(Debug, StructOpt)]
enum CliAction {
    /// [default] REPL mode
    Run,
    /// Searches for the provided query
    Query { query: String },
    /// Clears the index and pushes all documents
    Reindex,
}

#[derive(StructOpt)]
#[structopt(name = "fuzzy5e")]
/// Compendium of D&D 5e spells, monsters, etc
struct Cli {
    #[structopt(subcommand)]
    /// Optional command. If not provided, program will be run in REPL mode
    action: Option<CliAction>,

    #[structopt(short, long, default_value = "[::1]:1491", env = "SONIC_ADDR")]
    sonic_addr: String,

    #[structopt(short, long, default_value = "localhost:27017", env = "MONGO_ADDR")]
    mongo_addr: String,
}

enum Action {
    Quit,
    Backspace,
    DeleteWord,
    AddChar(char),
    SelectPrevious,
    SelectNext,
    SetLayout(Layout),
    ScrollUp(usize),
    ScrollDown(usize),
    Resize,
}

fn update_matches(idx: &Client, conn: &DB, query: &str, matches: Arc<Mutex<Vec<Model>>>) {
    if let Ok(mut matches) = matches.lock() {
        matches.clear();
        matches.extend(Model::indexed_query(idx, conn, query).unwrap());
    }
}

struct Selection(Option<Model>, usize);

impl Draw for Selection {
    fn draw(&self, canvas: &mut dyn Canvas) -> canvas::Result<()> {
        if let Some(m) = self.0.clone() {
            m.draw(canvas, self.1.clone())
        } else {
            Ok(())
        }
    }
}

impl Widget for Selection {}

struct Matches {
    matches: Arc<Mutex<Vec<Model>>>,
    selected: usize,
}

impl Draw for Matches {
    fn draw(&self, canvas: &mut dyn Canvas) -> canvas::Result<()> {
        let selected_attr = Attr {
            bg: Color::WHITE,
            fg: Color::BLACK,
            ..Attr::default()
        };
        if let Ok(matches) = self.matches.lock() {
            for (idx, result) in matches.iter().enumerate() {
                let (text, text_attr) = result.display_name();
                let (fmt_text, attr) = if self.selected == idx {
                    (format!("> {}", text), selected_attr)
                } else {
                    (format!("  {}", text), text_attr)
                };
                let _ = canvas.print_with_attr(idx, 0, &fmt_text, attr);
            }
        }
        Ok(())
    }
}

impl Widget for Matches {}

struct Input(Arc<Mutex<Query>>);
impl Draw for Input {
    fn draw(&self, canvas: &mut dyn Canvas) -> canvas::Result<()> {
        let placeholder = ("? Begin typing...", Attr::from(Color::LIGHT_BLUE));
        if let Ok(s) = self.0.lock() {
            if s.is_empty() {
                let _ = canvas.print_with_attr(0, 0, placeholder.0, placeholder.1);
            } else {
                let _ = canvas.print_with_attr(0, 0, &format!("? {}", s), Attr::default());
            }
            let _ = canvas.set_cursor(0, 2 + s.to_string().len());
        }
        Ok(())
    }
}

impl Widget for Input {}

fn key_to_action(ev: Event) -> Option<Action> {
    match ev {
        Event::Resize{..} => Some(Action::Resize),
        Event::Key(Key::ESC)
        | Event::Key(Key::Ctrl('c'))
        | Event::Key(Key::Ctrl('d'))
        | Event::Key(Key::Ctrl('q')) => Some(Action::Quit),
        Event::Key(Key::Backspace) => Some(Action::Backspace),
        Event::Key(Key::Char('/')) => Some(Action::SetLayout(Layout::Querying)),
        Event::Key(Key::Char(key)) => Some(Action::AddChar(key)),
        Event::Key(Key::Ctrl('w')) => Some(Action::DeleteWord),
        Event::Key(Key::Ctrl('n')) => Some(Action::SelectNext),
        Event::Key(Key::Ctrl('p')) => Some(Action::SelectPrevious),
        Event::Key(Key::Enter) => Some(Action::SetLayout(Layout::Selected)),
        Event::Key(Key::PageDown) => Some(Action::ScrollDown(10)),
        Event::Key(Key::PageUp) => Some(Action::ScrollUp(10)),
        Event::Key(Key::Down) => Some(Action::ScrollDown(1)),
        Event::Key(Key::Up) => Some(Action::ScrollUp(1)),
        _ => None,
    }
}

/// Layout determines the presentation of the screen.
#[derive(PartialEq)]
enum Layout {
    /// Querying means that the search input pane will be shown as well as the matches
    Querying,
    /// Selected means that only the last selected match will be shown (query & matches are hidden)
    Selected,
}

struct Screen5e {
    query: Arc<Mutex<Query>>,
    matches: Arc<Mutex<Vec<Model>>>,
    selected: usize,
    scroll: usize,
    term: Arc<Term>,
    layout: Layout,
}

enum Scroll {
    Up(usize),
    Down(usize),
}

/* This doesn't work, so added the "cleanup" method to the impl instead
impl Drop for Screen5e {
    fn drop(&mut self) {
        self.set_layout(Layout::Querying);
    }
}
*/

impl Screen5e {
    fn new(term: Arc<Term>, query: Arc<Mutex<Query>>, matches: Arc<Mutex<Vec<Model>>>) -> Screen5e {
        Screen5e {
            query,
            matches,
            selected: 0,
            scroll: 0,
            term,
            layout: Layout::Querying,
        }
    }

    fn cleanup(&mut self) {
        self.set_layout(Layout::Querying);
    }

    fn set_selected(&mut self, selected: usize) {
        self.selected = selected;
        self.update();
    }

    fn set_layout(&mut self, layout: Layout) {
        self.layout = layout;
        self.update();
    }

    fn select_next(&mut self) {
        let len = self.matches.lock().unwrap().len();
        if len > 0 && self.selected < len - 1 {
            self.selected += 1;
            self.scroll = 0;
            self.update();
        }
    }

    fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.scroll = 0;
            self.update();
        }
    }

    fn scroll(&mut self, scroll: Scroll) {
        match scroll {
            Scroll::Up(s) => {
                if self.scroll < s {
                    self.scroll = 0;
                } else {
                    self.scroll -= s;
                }
            }
            Scroll::Down(s) => self.scroll += s
        }
        self.update();
    }

    fn update(&self) {
        debug!("screen.update()");
        let _ = self.term.clear();

        let m = Matches {
            matches: Arc::clone(&self.matches),
            selected: self.selected,
        };
        let q = Input(Arc::clone(&self.query));
        let sel = {
            if let Ok(matches) = self.matches.lock() {
                if let Some(m) = matches.get(self.selected) {
                    Some(m.clone())
                } else {
                    None
                }
            } else {None}
        };
        let s = Selection(sel.clone(), self.scroll);

        match self.layout {
            Layout::Querying => {
                let split = VSplit::default()
                    .split(Win::new(&q).basis(Size::Fixed(1)))
                    .split(
                        HSplit::default()
                            .basis(Size::Percent(30))
                            .split(
                                Win::new(&m)
                                    .border(true)
                                    .basis(Size::Percent(30))
                                    .margin_top(1)
                                    .title("Results")
                                    .title_attr(Attr::from(Color::LIGHT_GREEN)),
                            )
                            .split(
                                Win::new(&s)
                                    .border(true)
                                    .basis(Size::Percent(70))
                                    .margin_top(1)
                                    .padding_left(1)
                                    .padding_right(1)
                                    .title("Selected")
                                    .title_attr(Attr::from(Color::LIGHT_GREEN)),
                            )
                            .basis(Size::Percent(100)),
                    );
                let _ = self.term.draw(&split);
                let _ = self.term.show_cursor(true);
            }
            Layout::Selected => {
                let title = if let Some(sl) = sel {
                    sl.display_name().0
                } else {
                    String::default()
                };
                let split = Win::new(&s)
                    .basis(Size::Percent(100))
                    .border(true)
                    .padding_left(1)
                    .padding_right(1)
                    .title(&title)
                    .title_attr(Attr::from(Color::LIGHT_GREEN));
                let _ = self.term.draw(&split);
                let _ = self.term.show_cursor(false);
            }
        }
        let _ = self.term.present();
        info!("done screen.update()");
    }
}

fn do_query(config: Config, query: &str) -> std::result::Result<(), Box<dyn Error>> {
    trace!("do_query");
    let db = DB::connect(&config.mongo_addr).expect("failed to connect to mongodb");
    let options = ClientOptions{ addr: config.sonic_addr.to_socket_addrs()?.next().unwrap(), ..ClientOptions::default()};
    let mut idx = Client::connect(options).expect("failed to connect to sonic");
    let results = Model::indexed_query(&idx, &db, &query);
    println!("{:?}", results);
    idx.disconnect()?;
    Ok(())
}

fn do_reindex(config: Config) -> std::result::Result<(), Box<dyn Error>> {
    trace!("do_query");
    let db = DB::connect(&config.mongo_addr).expect("failed to connect to mongodb");
    let options = ClientOptions {
        addr: config.sonic_addr.to_socket_addrs()?.next().unwrap(),
        mode: Mode::Ingest,
        ..ClientOptions::default()
    };
    let mut idx = Client::connect(options).expect("failed to connect to sonic");

    Model::flush_all(&idx)?;
    Model::index_all(&idx, &db)?;

    idx.disconnect()?;
    Ok(())
}

#[derive(Clone,Debug)]
struct Query {
    inner: String
}
impl Query {
    fn new() -> Self {
        Query {inner: String::default()}
    }
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    fn push(&mut self, ch: char) {
        self.inner.push(ch);
    }
    fn backspace(&mut self) {
        self.inner.pop();
    }
    fn delete_word(&mut self) {
        let mut didpop = false;
        while !self.is_empty() {
            if self.inner.pop().unwrap() == ' ' && didpop {
                self.inner.push(' ');
                break;
            }
            didpop = true;
        }
    }
}
impl Display for Query {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

fn do_run(config: Config) -> std::result::Result<(), Box<dyn Error>> {
    let query = Arc::new(Mutex::new(Query::new()));
    let q2 = Arc::clone(&query);

    let matches = Arc::new(Mutex::new(Vec::new()));
    // let m2 = Arc::clone(&matches);

    // Term is thread-safe
    let term = Arc::new(Term::new().unwrap());

    let screen = Arc::new(Mutex::new(Screen5e::new(Arc::clone(&term), Arc::clone(&query), Arc::clone(&matches))));
    let sc2 = Arc::clone(&screen);

    let _ = thread::spawn(move || {
        // NOTE: Keep these connections inside the thread. For some reason starting
        // these from the main thread and then moving them prevents tuikit from
        // receiving WINCH signals :/
        let db = DB::connect(&config.mongo_addr).expect("This should not fail");
        let idx = Client::connect(ClientOptions{addr: config.sonic_addr.to_socket_addrs().unwrap().next().unwrap(), ..ClientOptions::default()}).expect("failed to connect to sonic");
        let mut last = String::default();
        loop {
            let q = if let Ok(query) = query.lock() {
                query.to_string()
            } else {
                last.clone()
            };

            if q != last && !q.is_empty() {
                debug!("query is different");
                update_matches(&idx, &db, &q, Arc::clone(&matches));
                last.clear();
                last.push_str(&q);
                if let Ok(sc2) = sc2.lock() {
                    info!("update loop got screen lock");
                    sc2.update();
                    info!("releasing screen lock");
                }
            }
            thread::sleep(std::time::Duration::from_millis(100));
        }
    });

    let th = thread::spawn(move || {
        if let Ok(screen) = screen.lock() {
            screen.update();
        }
        loop {
            let action = key_to_action(term.poll_event().unwrap());

            debug!("loop got action");
            if let Some(a) = action {
                match a {
                    Action::Quit => {
                        if let Ok(mut screen) = screen.lock() {
                            if screen.layout == Layout::Selected {
                                screen.set_layout(Layout::Querying);
                            } else {
                                screen.cleanup();
                                break;
                            }
                        }
                    }
                    Action::Backspace => {
                        if let Ok(mut screen) = screen.lock() {
                            if screen.layout == Layout::Selected {
                                continue;
                            }
                            q2.lock().unwrap().backspace();
                            screen.set_selected(0);
                        }

                    }
                    Action::DeleteWord => {
                        if let Ok(screen) = screen.lock() {
                            if screen.layout == Layout::Selected {
                                continue;
                            }
                            q2.lock().unwrap().delete_word();
                            screen.update();
                        }
                    }
                    Action::AddChar(key) => {
                        if let Ok(mut screen) = screen.lock() {
                            if screen.layout == Layout::Selected {
                                continue;
                            }
                            q2.lock().unwrap().push(key);
                            screen.set_selected(0);
                        }

                    }
                    Action::ScrollUp(n) => {
                        if let Ok(mut screen) = screen.lock() {
                            screen.scroll(Scroll::Up(n));
                        }
                    }
                    Action::ScrollDown(n) => {
                        if let Ok(mut screen) = screen.lock() {
                            screen.scroll(Scroll::Down(n));
                        }
                    }
                    Action::SelectNext => {
                        if let Ok(mut screen) = screen.lock() {
                            if screen.layout == Layout::Selected {
                                continue;
                            }
                            screen.select_next();
                        }
                    }
                    Action::SelectPrevious => {
                        if let Ok(mut screen) = screen.lock() {
                            if screen.layout == Layout::Selected {
                                continue;
                            }
                            screen.select_prev();
                        }
                    }
                    Action::SetLayout(l) => {
                        if let Ok(mut screen) = screen.lock() {
                            screen.set_layout(l);
                        }
                    }
                    Action::Resize => {
                        if let Ok(screen) = screen.lock() {
                            screen.update();
                        }
                    }
                }
            }
        }
    });
    if th.join().is_err() {
        Err(Box::new(RuntimeError::Unexpected(String::from(
            "main thread loop panicked; check db and index services are running",
        ))))
    } else {
        Ok(())
    }
}

/// Checks sonic to see if docs need to be indexed
fn indexing_required(idx: &Client, conn: &DB) -> bool {
    if let Ok(res) = Model::indexed_query(idx, conn, "fire") {
        res.is_empty()
    } else {
        true
    }
}

fn main() -> std::result::Result<(), Box<dyn Error>> {
    let level = if let Ok(level) = std::env::var("RUST_LOG") {
        if let Ok(lf) = simplelog::LevelFilter::from_str(&level) {
            lf
        } else { simplelog::LevelFilter::Off }
    } else {
        simplelog::LevelFilter::Off
    };
    simplelog::WriteLogger::init(level, simplelog::Config::default(), File::create("output.log").unwrap())?;

    let cli = Cli::from_args();

    let config = Config {
        sonic_addr: cli.sonic_addr,
        mongo_addr: cli.mongo_addr,
    };


    // TODO: First time run needs to ask about building the index and do it!

    match cli.action {
        Some(action) => match action {
            CliAction::Run => do_run(config),
            CliAction::Query { query } => do_query(config, &query),
            CliAction::Reindex => do_reindex(config),
        },
        None => do_run(config),
    }
}

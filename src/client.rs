use crate::worker::TcpStreamWorker;
use derivative::Derivative;
// use log::{debug, info, warn};
// use std::collections::VecDeque;
use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::net::SocketAddr;
use std::result::Result;
// use std::slice::Split;
use std::sync::{
    mpsc::{channel, Receiver, RecvTimeoutError, Sender},
    Arc, Condvar, Mutex,
};
use std::thread::{spawn, JoinHandle};
use std::time::Duration;

#[derive(Debug)]
pub enum ClientError {
    // ConnectionError,
    ProcessingError,
}

impl Display for ClientError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)?;
        Ok(())
    }
}

impl Error for ClientError {}

pub trait Message: Display {}

pub enum Mode {
    Search,
    Control,
    Ingest,
}

impl Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Search => "search",
                Self::Control => "control",
                Self::Ingest => "ingest",
            }
        )
    }
}

pub enum RequestMessage<'a> {
    Start(Mode, &'a str),
    Quit,
}

impl Display for RequestMessage<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Start(mode, pw) => format!("START {} {}", mode, pw),
                Self::Quit => String::from("QUIT"),
            }
        )
    }
}

impl Message for RequestMessage<'_> {}

#[derive(Debug)]
pub enum ResponseMessage {
    Connected,
    Started,
    Ended,
    Pending(Pending),
    Event(Event),
    Result,
    Ok,
    Err,
}

pub enum IngestRequestMessage<'a> {
    Push(&'a str, &'a str, &'a str, &'a str),
    Flushc(&'a str),
}

impl Message for IngestRequestMessage<'_> {}
impl Display for IngestRequestMessage<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Push(collection, bucket, object, text) =>
                    format!("PUSH {} {} {} \"{}\"", collection, bucket, object, text),
                Self::Flushc(collection) => format!("FLUSHC {}", collection),
            }
        )
    }
}

pub enum SearchRequestMessage<'a> {
    Query(&'a str, &'a str, &'a str),
}

impl Display for SearchRequestMessage<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Query(collection, bucket, query) =>
                    format!("QUERY {} {} \"{}\"", collection, bucket, query),
            }
        )
    }
}

impl Message for SearchRequestMessage<'_> {}

#[derive(Debug)]
pub enum EventKind {
    Query,
    Suggest,
}

impl From<&str> for EventKind {
    fn from(s: &str) -> Self {
        match s {
            "QUERY" => Self::Query,
            "SUGGEST" => Self::Suggest,
            &_ => unreachable!(),
        }
    }
}

#[derive(Debug)]
pub struct Event {
    pub id: String,
    pub kind: EventKind,
    pub data: Vec<String>,
    raw: String,
}

#[derive(Debug)]
pub struct Pending {
    id: String,
    raw: String,
}

impl From<String> for ResponseMessage {
    fn from(s: String) -> Self {
        let mut tokens: Vec<String> = s.split(' ').map(String::from).collect();
        match &*tokens.remove(0) {
            "ENDED" => ResponseMessage::Ended,
            "CONNECTED" => ResponseMessage::Connected,
            "STARTED" => ResponseMessage::Started,
            "PENDING" => ResponseMessage::Pending(Pending {
                id: tokens.remove(0),
                raw: s,
            }),
            "EVENT" => {
                let kind = EventKind::from(&*tokens.remove(0));
                let id = tokens.remove(0);
                ResponseMessage::Event(Event {
                    kind,
                    id,
                    data: tokens,
                    raw: s,
                })
            }
            "RESULT" => ResponseMessage::Result,
            "OK" => ResponseMessage::Ok,
            "ERR" => ResponseMessage::Err,
            s => unreachable!(s),
        }
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
struct InnerClient {
    results_rx: Receiver<String>,
    tx: Sender<String>,
    event_loop: Option<JoinHandle<()>>,
    started: Arc<(Mutex<bool>, Condvar)>,
}

/// Client accepts queries / commands destined for sonic server and
/// dispatches via underlying thread.
///
/// # Examples
///
/// ```
/// let mut client = Client::connect(ClientOptions::default()).expect("Failed to connect");
/// let col = "mycollection";
/// let bkt = "mybucket";
/// let qry = "Find this text";
/// let res = client.send(SearchRequestMessage::Query(col, bkt, qry)).expect("Query failed");
/// for r in res.data {
///     // Do something interesting
/// }
/// let _ = client.disconnect();
/// ```
#[derive(Clone, Debug)]
pub struct Client {
    inner: Arc<Mutex<InnerClient>>,
}

/// Settings for connecting to sonic
pub struct ClientOptions<'a> {
    pub addr: SocketAddr,
    pub password: &'a str,
    pub mode: Mode,
}

impl Default for ClientOptions<'_> {
    fn default() -> Self {
        Self {
            addr: "[::1]:1491".parse().unwrap(),
            password: "SecretPassword",
            mode: Mode::Search,
        }
    }
}

impl Client {
    pub fn connect(options: ClientOptions) -> Result<Self, Box<dyn Error>> {
        // tcp
        let (tcp_tx, tcp_rx) = channel();

        let (tx, rx): (Sender<String>, Receiver<String>) = channel();

        let (results_tx, results_rx) = channel();

        let mut worker = TcpStreamWorker::connect(options.addr, tcp_tx)?;

        let pair = Arc::new((Mutex::new(false), Condvar::new()));
        let pair2 = pair.clone();

        // Communicates with worker
        let event_loop = spawn(move || {
            'event_loop: loop {
                // Control signals??
                {}

                // Sends all pending messages in queue
                loop {
                    match rx.recv_timeout(Duration::from_millis(10)) {
                        Ok(msg) => {
                            debug!("[client-send] writing: {}", msg);
                            if let Err(err) = worker.write(&msg) {
                                trace!("[client-send] error writing: {}", err);
                            }
                        }
                        Err(err) => match err {
                            RecvTimeoutError::Timeout => {
                                trace!("[client-send] (timeout)    {}", err);
                                break;
                            }
                            RecvTimeoutError::Disconnected => {
                                trace!("[client-send] (disconnect) {}", err);
                                break 'event_loop;
                            }
                        },
                    }
                }

                // Receives all pending results
                loop {
                    match tcp_rx.recv_timeout(Duration::from_millis(10)) {
                        Ok(res) => {
                            debug!("[client-recv] result: {}", res);
                            let _ = match ResponseMessage::from(res.clone()) {
                                ResponseMessage::Started => {
                                    debug!("Got Started");
                                    let (lock, cvar) = &*pair;
                                    let mut started = lock.lock().unwrap();
                                    *started = true;
                                    cvar.notify_all();
                                    continue;
                                }
                                ResponseMessage::Connected => continue,
                                ResponseMessage::Ended => break,
                                ResponseMessage::Event(_) => results_tx.send(res),
                                ResponseMessage::Pending(_) => continue,
                                ResponseMessage::Result => results_tx.send(res),
                                ResponseMessage::Ok => results_tx.send(res),
                                ResponseMessage::Err => {
                                    error!("{}", res);
                                    let _ = results_tx.send(res);
                                    break;
                                }
                            };
                        }
                        Err(err) => match err {
                            RecvTimeoutError::Timeout => {
                                trace!("[client-recv] (timeout)    {}", err);
                                break;
                            }
                            RecvTimeoutError::Disconnected => {
                                trace!("[client-recv] (disconnect) {}", err);
                                break 'event_loop;
                            }
                        },
                    }
                }
            }
            if let Err(err) = worker.disconnect() {
                trace!("Error disconnecting TcpStreamWorker {}", err);
            }
            trace!("Wrapping up");
        });

        let client = Client {
            inner: Arc::new(Mutex::new(InnerClient {
                event_loop: Some(event_loop),
                tx,
                results_rx,
                started: pair2,
            })),
        };

        client.write(&RequestMessage::Start(options.mode, options.password).to_string())?;
        Ok(client)
    }

    /// Sends message to TcpStreamWorker and does not wait or block.
    // pub fn send_nowait(&self, m: impl Message) -> Result<(), Box<dyn Error>> {
    //     // Wait for worker to start up if necessary
    //     {
    //         let (lock, cvar) = &*self.inner.lock().unwrap().started;
    //         let mut started = lock.lock().unwrap();
    //         while !*started {
    //             debug!("send called but waiting for start");
    //             started = cvar.wait(started).unwrap();
    //         }
    //     }
    //
    //     // Send message to worker
    //     self.write(&format!("{}\r\n", m))?;
    //
    //     // Await response
    //     Ok(())
    // }
    //
    /// Sends message to TcpStreamWorker and awaits response. Messages are
    /// processed in-order of receipt.
    pub fn send(&self, m: impl Message) -> Result<Box<ResponseMessage>, Box<dyn Error>> {
        // Wait for worker to start up if necessary
        {
            let (lock, cvar) = &*self.inner.lock().unwrap().started;
            let mut started = lock.lock().unwrap();
            while !*started {
                debug!("send called but waiting for start");
                started = cvar.wait(started).unwrap();
            }
        }

        // Send message to worker
        let _ = self.write(&format!("{}\r\n", m));

        // Await response
        Ok({
            let inner = self.inner.lock().unwrap();
            let res: String = inner.results_rx.recv()?;
            Box::new(ResponseMessage::from(res))
        })
    }

    /// Writes message to TcpStreamWorker. Does not guarantee delivery
    /// or response. Does not block.
    fn write(&self, msg: &str) -> Result<(), Box<dyn Error>> {
        trace!("write");
        if self
            .inner
            .lock()
            .unwrap()
            .tx
            .send(String::from(msg))
            .is_ok()
        {
            Ok(())
        } else {
            Err(Box::new(ClientError::ProcessingError))
        }
    }

    /// Terminates the event loop and the underlying TcpStream.
    /// Before the event loop thread ends it calls disconnect
    /// on the TcpStream worker.
    pub fn disconnect(&mut self) -> Result<(), Box<dyn Error>> {
        trace!("disconnect");
        let mut inner = self.inner.lock().unwrap();
        {
            let _ = inner.tx.send(String::from("QUIT\r\n"));
            std::thread::sleep(Duration::from_secs(1));
        }

        // Waits for event loop thread to terminate
        let _ = inner.event_loop.take().unwrap().join();

        Ok(())
    }
}

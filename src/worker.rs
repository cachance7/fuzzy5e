// use log::{debug, info, warn};
use std::error::Error;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::net::{SocketAddr, TcpStream};
use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Arc, Mutex,
};
use std::thread::{spawn, JoinHandle};

// use mio::net::TcpStream;
// use mio::{Events, Interest, Poll, Token};
use std::io::{BufRead, BufReader, BufWriter, Write};

// Some tokens to allow us to identify which event is for which socket.
// const CLIENT: Token = Token(0);

#[derive(Debug)]
pub enum WorkerError {
    // ConnectionError,
    ProcessingError(Box<dyn Error>),
    ChannelClosedError,
    ThreadError,
}

impl Display for WorkerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)?;
        Ok(())
    }
}

impl Error for WorkerError {}

#[derive(Debug)]
struct InnerWorker {
    tx: Option<Sender<String>>,
    r_thread: Option<JoinHandle<()>>,
    w_thread: Option<JoinHandle<()>>,
    socket: TcpStream,
}

#[derive(Debug)]
pub struct TcpStreamWorker {
    inner: Arc<Mutex<InnerWorker>>,
}

impl TcpStreamWorker {
    pub fn connect(addr: SocketAddr, results: Sender<String>) -> Result<Self, Box<dyn Error>> {
        let (tx, rx): (Sender<String>, Receiver<String>) = channel();

        // Setup the client socket.
        let socket = TcpStream::connect(addr)?;
        Ok(TcpStreamWorker {
            inner: Arc::new(Mutex::new(InnerWorker {
                tx: Some(tx),
                w_thread: Some(TcpStreamWorker::start_writer(socket.try_clone()?, rx)),
                r_thread: Some(TcpStreamWorker::start_reader(socket.try_clone()?, results)),
                socket,
            })),
        })
    }

    /// Reads messages from provided Receiver and writes to socket
    fn start_writer(socket: TcpStream, rx: Receiver<String>) -> JoinHandle<()> {
        let mut writer = BufWriter::new(socket);
        spawn(move || {
            // Start an event loop.
            trace!("[tcpstream-send] event_loop start");
            loop {
                match rx.recv() {
                    Ok(msg) => {
                        debug!("[tcpstream-send] writing: {}", msg);
                        if let Err(err) = writer.write(msg.as_bytes()) {
                            trace!("[tcpstream-send] error writing {}", err);
                        }
                        let _ = writer.flush();
                    }
                    Err(err) => {
                        warn!("[tcpstream-send] disconnected: {}", err);
                        break;
                    }
                }
            }
            trace!("[tcpstream-send] event_loop end");
        })
    }

    /// Reads from provided socket and writes read lines to out sender
    fn start_reader(socket: TcpStream, out: Sender<String>) -> JoinHandle<()> {
        let mut reader = BufReader::new(socket);
        spawn(move || {
            trace!("[tcpstream-recv] event_loop start");
            loop {
                let mut buf = String::default();
                match reader.read_line(&mut buf) {
                    Ok(len) => {
                        trace!("[tcpstream-recv] read {} bytes", len);
                        if len == 0 {
                            trace!("[tcpstream-recv] break");
                            break;
                        } else {
                            let b = buf.trim().to_string();
                            debug!("[tcpstream-recv] sending result: {}", b);
                            if let Err(err) = out.send(b) {
                                trace!("[tcpstream-recv] failed to send results {}", err);
                            }
                        }
                    }
                    Err(err) => {
                        warn!("[tcpstream-recv] bufreader quit: {}", err);
                        break;
                    }
                };
            }
            trace!("[tcpstream-recv] event_loop end");
        })
    }

    pub fn write(&self, msg: &str) -> Result<(), WorkerError> {
        trace!("write: {}", msg);
        match &self.inner.lock().unwrap().tx {
            Some(sender) => match sender.send(String::from(msg)) {
                Ok(_) => Ok(()),
                Err(err) => Err(WorkerError::ProcessingError(Box::from(err))),
            },
            None => Err(WorkerError::ChannelClosedError),
        }
    }

    pub fn disconnect(&mut self) -> Result<(), WorkerError> {
        trace!("disconnecting");
        self.inner.lock().unwrap().disconnect()
    }
}

impl InnerWorker {
    fn disconnect(&mut self) -> Result<(), WorkerError> {
        trace!("InnerWorker disconnecting");
        drop(self.tx.take());
        let _ = match self.w_thread.take().unwrap().join() {
            Ok(_) => Ok(()),
            Err(_) => Err(WorkerError::ThreadError),
        };
        let _ = self.socket.shutdown(std::net::Shutdown::Both);
        match self.r_thread.take().unwrap().join() {
            Ok(_) => Ok(()),
            Err(_) => Err(WorkerError::ThreadError),
        }
    }
}

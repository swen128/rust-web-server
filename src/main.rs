use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    sync::mpsc,
    thread,
    time::Duration,
};

use hello::ThreadPool;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    let local_addr = listener.local_addr().unwrap();
    let pool = ThreadPool::new(4);

    // When the user presses Ctrl-C, the loop will break and it starts the graceful shutdown.
    let (tx, rx) = mpsc::channel::<String>();
    ctrlc::set_handler(move || {
        tx.send("shutdown".to_string()).unwrap();

        // Connect to the server to unblock the listener.accept() call.
        TcpStream::connect(local_addr).unwrap();
    })
    .unwrap();

    for stream in listener.incoming() {
        match rx.try_recv() {
            // Ctrl-C hasn't been pressed.
            Err(mpsc::TryRecvError::Empty) => {
                let stream = stream.unwrap();
                pool.execute(|| {
                    handle_connection(stream);
                });
            }

            Ok(_) | Err(mpsc::TryRecvError::Disconnected) => {
                println!("Shutting down.");
                break;
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);
    let request_first_line = buf_reader
        .lines()
        .next()
        .unwrap_or(Ok("".to_string()))
        .unwrap();

    let (status_line, filename) = match request_first_line.as_str() {
        "GET / HTTP/1.1" => ("HTTP/1.1 200 OK", "hello.html"),
        "GET /sleep HTTP/1.1" => {
            thread::sleep(Duration::from_secs(5));
            ("HTTP/1.1 200 OK", "hello.html")
        }
        _ => ("HTTP/1.1 404 Not Found", "404.html"),
    };

    let contents = fs::read_to_string(filename).unwrap();
    let length = contents.len();
    let headers = format!("Content-Length: {length}");

    let response = format!("{status_line}\r\n{headers}\r\n\r\n{contents}");

    stream.write_all(response.as_bytes()).unwrap();
}

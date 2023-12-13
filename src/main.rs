// main.rs
// modified from the manpage example
// the page_fault_handler now monitors the PF in child processes
// author: Hank Bao

use std::convert::TryInto;
use std::net::Shutdown;
use std::os::fd::{AsFd, AsRawFd, FromRawFd};
use std::os::unix::net::{UnixListener, UnixStream};
use std::{env, fs, os, process, slice, thread, time};

use libc::{self, c_void};
use nix::poll::{poll, PollFd, PollFlags};
use nix::sys::mman::{mmap, MapFlags, ProtFlags};
use nix::sys::signal::{sigprocmask, SigSet, SigmaskHow, Signal};
use nix::sys::signalfd::{SfdFlags, SignalFd};
use nix::unistd::{fork, getpid, sysconf, ForkResult, SysconfVar};
use sendfd::{RecvWithFd, SendWithFd};
use userfaultfd::{Event, Uffd, UffdBuilder};

fn page_fault_handler(uffd: Uffd, page_size: usize) {
    // Block SIGCHLD
    let mut sigset = SigSet::empty();
    sigset.add(Signal::SIGCHLD);
    if let Err(e) = sigprocmask(SigmaskHow::SIG_BLOCK, Some(&sigset), None) {
        die("sigprocmask()", e);
    }

    // Create a signalfd for SIGCHLD
    let mut sigfd =
        match SignalFd::with_flags(&sigset, SfdFlags::SFD_NONBLOCK | SfdFlags::SFD_CLOEXEC) {
            Ok(sf) => sf,
            Err(e) => die("SignalFd::with_flags()", e),
        };

    // Create a page that will be copied into the faulting region
    let page = unsafe { mmap_anon(page_size) };

    // Loop, handling incoming events on the userfaultfd file descriptor
    let mut fault_cnt = 0;
    loop {
        // See what poll() tells us about the userfaultfd
        let mut fds = [
            PollFd::new(&uffd, PollFlags::POLLIN),
            PollFd::new(&sigfd, PollFlags::POLLIN),
        ];
        if let Err(e) = poll(&mut fds, -1) {
            die("poll", e);
        }

        for pollfd in &fds {
            let revents = match pollfd.revents() {
                Some(e) => e,
                None => die("pollfd.revents()", "returned None unexpectedly"),
            };
            if !revents.contains(PollFlags::POLLIN) {
                continue;
            }

            if pollfd.as_fd().as_raw_fd() == uffd.as_raw_fd() {
                // Read an event from the userfaultfd
                let event = match uffd.read_event() {
                    Ok(Some(e)) => e,
                    Ok(None) => die("uffd.read_event()", "returned None after poll() notified"),
                    Err(e) => die("uffd.read_event()", e),
                };

                // We expect only one kind of event; verify that assumption
                if let Event::Pagefault { addr, .. } = event {
                    // Display info about the page-fault event

                    println!(
                        "<pid:{}>    UFFD_EVENT_PAGEFAULT event: {:?}",
                        getpid(),
                        event
                    );

                    // Copy the page pointed to by 'page' into the faulting region. Vary the contents that are
                    // copied in, so that it is more obvious that each fault is handled separately.
                    for c in unsafe { slice::from_raw_parts_mut(page as *mut u8, page_size) } {
                        *c = b'A' + fault_cnt % 20;
                    }
                    fault_cnt += 1;

                    let dst = (addr as usize & !(page_size - 1)) as *mut c_void;
                    let copied = unsafe {
                        match uffd.copy(page, dst, page_size, true) {
                            Ok(size) => size,
                            Err(e) => die("uffd.copy()", e),
                        }
                    };

                    println!(
                        "<pid:{}>        (uffdio_copy.copy returned {})",
                        getpid(),
                        copied
                    );
                } else {
                    die("uffd.read_event", format!("unexpected event {:?}", event));
                }
            } else if pollfd.as_fd().as_raw_fd() == sigfd.as_raw_fd() {
                match sigfd.read_signal() {
                    Ok(Some(siginfo)) => {
                        assert!(siginfo.ssi_signo == Signal::SIGCHLD as u32);
                        println!(
                            "<pid:{}> got signal SIGCHLD from child {}",
                            getpid(),
                            siginfo.ssi_pid
                        );
                        return;
                    }
                    Ok(None) => die("sigfd.read_signal()", "returned None after poll() notified"),
                    Err(e) => die("sigfd.read_signal()", e),
                }
            }
        }
    }
}

fn usage() -> ! {
    println!("Usage: paging <num_pages>");
    process::exit(1)
}

fn main() {
    let num_pages = if let Some(n) = env::args().nth(1) {
        match n.parse::<usize>() {
            Ok(num) => num,
            Err(_) => usage(),
        }
    } else {
        usage();
    };

    println!("<pid:{}> parent is entering...", getpid());

    let page_size = match sysconf(SysconfVar::PAGE_SIZE) {
        Ok(Some(size)) => {
            println!("<pid:{}> PAGE_SIZE is {}", getpid(), size);
            size as usize
        }
        Ok(None) => {
            let default_size: usize = 4096;
            eprintln!(
                "<pid:{}> sysconf(PAGE_SIZE) not set. Use default size {}",
                getpid(),
                default_size
            );
            default_size
        }
        Err(e) => die("sysconf()", e),
    };

    let len = num_pages * page_size;
    let sock_path = "/tmp/uffd.sock".to_owned();

    // Create a child process to access the memory
    match unsafe { fork() } {
        Ok(ForkResult::Parent { .. }) => {
            println!("<pid:{}> forked", getpid());

            let listener = match UnixListener::bind(&sock_path) {
                Ok(l) => l,
                Err(e) => die("UnixListener::bind()", e),
            };
            let (stream, _) = match listener.accept() {
                Ok(s) => s,
                Err(e) => die("UnixListener::accpet()", e),
            };

            let uffd = get_uffd(stream);
            page_fault_handler(uffd, page_size);

            drop(listener);
            if let Err(e) = fs::remove_file(sock_path) {
                die("fs::remove_file()", e);
            } else {
                println!("<pid:{}> parent is exiting...", getpid());
                process::exit(0);
            }
        }
        Ok(ForkResult::Child) => {
            child(len, &sock_path);
        }
        Err(e) => {
            die("fork()", e);
        }
    }
}

unsafe fn mmap_anon(len: usize) -> *mut c_void {
    match mmap(
        None,
        len.try_into().expect("non-zero"),
        ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
        MapFlags::MAP_ANONYMOUS | MapFlags::MAP_PRIVATE,
        None::<os::fd::BorrowedFd>,
        0,
    ) {
        Ok(addr) => addr,
        Err(e) => die("mmap()", e),
    }
}

fn get_uffd(stream: UnixStream) -> Uffd {
    let mut fds = [0];
    let mut buff = vec![0; 64];
    match stream.recv_with_fd(&mut buff, &mut fds) {
        Ok((_, n)) => {
            if n == 1 {
                unsafe { Uffd::from_raw_fd(fds[0]) }
            } else {
                die(
                    "UnixStream::recv_with_fd()",
                    format!("it returned {} fds", n),
                );
            }
        }
        Err(e) => die("UnixStream::recv_with_fd()", e),
    }
}

fn child(len: usize, sock_path: &String) {
    println!("<pid:{}> child is entering...", getpid());

    // Create and enable userfaultfd object
    let uffd = match UffdBuilder::new()
        .close_on_exec(true)
        .non_blocking(true)
        .user_mode_only(true)
        .create()
    {
        Ok(u) => u,
        Err(e) => die("UffdBuilder::create()", e),
    };

    // Create a private anonymous mapping. The memory will be demand-zero paged--that is, not yet
    // allocated. When we actually touch the memory, it will be allocated via the userfaultfd.
    let addr = unsafe { mmap_anon(len) };

    println!("<pid:{}> address returned by mmap() = {:p}", getpid(), addr);

    // Register the memory range of the mapping we just created for handling by the userfaultfd
    // object. In mode, we request to track missing pages (i.e., pages that have not yet been
    // faulted in).

    if let Err(e) = uffd.register(addr, len) {
        die("uffd.register()", e);
    }

    let socket = match UnixStream::connect(sock_path) {
        Ok(s) => s,
        Err(e) => die("UnixStream::connect()", e),
    };

    if let Err(e) = socket.send_with_fd(&mut vec![0; 64], &[uffd.as_raw_fd()]) {
        die("socket.send_with_fd()", e);
    }

    if let Err(e) = socket.shutdown(Shutdown::Both) {
        die("socket.shutdown()", e);
    }

    // Children now touch memory in the mapping, touching locations 1024 bytes apart. This will
    // trigger userfaultfd events for all pages in the region.

    // Ensure that faulting address is not on a page boundary, in order to test that we correctly
    // handle that case in page_fault_handler()
    let mut l = 0xf;
    let delta = 1024;

    while l < len {
        let ptr = (addr as usize + l) as *mut u8;
        let c = unsafe { *ptr };
        println!("<pid:{}> read address {:p}: {:?}", getpid(), ptr, c as char);

        let to_sleep = time::Duration::from_micros(100000);
        thread::sleep(to_sleep);

        l += delta;
    }

    println!("<pid:{}> child is exiting...", getpid());
    process::exit(0);
}

fn die<E: std::fmt::Display>(reason: &str, error: E) -> ! {
    eprintln!("<pid:{}> {} failed {}", getpid(), reason, error);
    process::exit(1)
}

# CS5600 Final Project

## Abstract

This report presents the development and implementation of a user space paging program in Rust, an innovative project leveraging the capabilities of `userfaultfd` and `signalfd` within the Linux operating system. The primary objective of this project was to explore and harness the potential of `userfaultfd` for monitoring and handling page faults occurring in child processes. Paging normally happens in the kernel level, but relatively underused in conventional programming practices. By intercepting page faults at the user level, the program demonstrates a feasible method for managing memory access and handling in a multi-process environment.

A significant part of the project involved utilizing `userfaultfd` to detect page faults in child processes. This mechanism allowed for the control of page fault handling from the parent process, enabling a responsive and customizable reaction to memory access patterns in child processes. The implementation focused on creating user-defined responses to page faults, such as providing copies of data from specified locations, thereby introducing a layer of flexibility and control in memory management.

In conjunction with `userfaultfd`, the project also employed `signalfd` for efficient signal handling, specifically focusing on the `SIGCHLD` signal. This integration aimed to monitor and respond to state changes in child processes, such as termination or interruption, thereby enhancing the robustness and reliability of the parent process in managing its child processes. The use of `signalfd` showcased an effective method for signal management in user space, reducing the complexity and overhead typically associated with signal handling in multi-threaded applications.

Overall, this project demonstrates the practical applications and benefits of `userfaultfd` and `signalfd` in complex memory and process management scenarios. The insights gained from this exploration have significant implications for the development of efficient, reliable, and scalable system-level applications in Linux environments. The techniques and methodologies developed in this project pave the way for further research and innovation in user space memory management and inter-process communication mechanisms.

## Usage

```zsh
$ cargo run
    Finished dev [unoptimized + debuginfo] target(s) in 0.02s
     Running `target/debug/paging`
Usage: paging <num_pages>
```

## Design and Implementation

This project demonstrates an use of `userfaultfd` and `signalfd` to handle page faults and signals, respectively, in a multi-process context. The main goal is to monitor page faults (**PF**) in child processes and handle `SIGCHLD` signals in the parent process. This involves setting up communication between the parent and child processes and managing shared resources effectively.

### Setup `userfaultfd`

A `userfaultfd` object is created and configured in the child processes. This object allows the program to intercept and handle page faults in user space. In my design, the `userfaultfd` file descriptor (fd) from the child is passed to the parent process, enabling the parent to handle page faults occurring in the child process.

### Allocate memory with `mmap`

I use `mmap` to allocate memory regions in both parent and child processes. The memory is set up as private and anonymous, demand-zero paged, meaning it will be allocated upon access. This behavior of the memory region is crucial for the program to detect page faults in the child process.

### Memory access in child processes

The child process intentionally access the memory region to trigger page faults. These faults are then caught and handled by the parent process through the `userfaultfd` mechanism.

### Handle signals with `signalfd`

A `signalfd` is set up to handle `SIGCHLD` signals, which are sent to the parent process when a child process terminates. The signal file descriptor is monitored alongside the userfaultfd using the `poll` system call.

### FD passing with UNIX domain socket

A Unix domain socket is used to pass the `userfaultfd` file descriptor from the child process to the parent process. This is a crucial step as it allows the parent process to handle page faults for the child.

### Handling loop

The parent process enters a loop where it uses `poll` to wait for events on both the `userfaultfd` and `signalfd`. When a page fault occurs, the parent process handles it by providing a memory page. When a `SIGCHLD` signal is received, the parent process handles the signal.

## Test Run

### Output

```zsh
$ cargo run -- 10
    Finished dev [unoptimized + debuginfo] target(s) in 0.02s
     Running `target/debug/paging 10`
<pid:20295> parent is entering...
<pid:20295> PAGE_SIZE is 4096
<pid:20295> forked
<pid:20297> child is entering...
<pid:20297> address returned by mmap() = 0x7fd14a77d000
<pid:20295>    UFFD_EVENT_PAGEFAULT event: Pagefault { kind: Missing, rw: Read, addr: 0x7fd14a77d000 }
<pid:20295>        (uffdio_copy.copy returned 4096)
<pid:20297> read address 0x7fd14a77d00f: 'A'
<pid:20297> read address 0x7fd14a77d40f: 'A'
<pid:20297> read address 0x7fd14a77d80f: 'A'
<pid:20297> read address 0x7fd14a77dc0f: 'A'
<pid:20295>    UFFD_EVENT_PAGEFAULT event: Pagefault { kind: Missing, rw: Read, addr: 0x7fd14a77e000 }
<pid:20295>        (uffdio_copy.copy returned 4096)
<pid:20297> read address 0x7fd14a77e00f: 'B'
<pid:20297> read address 0x7fd14a77e40f: 'B'
<pid:20297> read address 0x7fd14a77e80f: 'B'
<pid:20297> read address 0x7fd14a77ec0f: 'B'
<pid:20295>    UFFD_EVENT_PAGEFAULT event: Pagefault { kind: Missing, rw: Read, addr: 0x7fd14a77f000 }
<pid:20295>        (uffdio_copy.copy returned 4096)
<pid:20297> read address 0x7fd14a77f00f: 'C'
<pid:20297> read address 0x7fd14a77f40f: 'C'
<pid:20297> read address 0x7fd14a77f80f: 'C'
<pid:20297> read address 0x7fd14a77fc0f: 'C'
<pid:20295>    UFFD_EVENT_PAGEFAULT event: Pagefault { kind: Missing, rw: Read, addr: 0x7fd14a780000 }
<pid:20295>        (uffdio_copy.copy returned 4096)
<pid:20297> read address 0x7fd14a78000f: 'D'
<pid:20297> read address 0x7fd14a78040f: 'D'
<pid:20297> read address 0x7fd14a78080f: 'D'
<pid:20297> read address 0x7fd14a780c0f: 'D'
<pid:20295>    UFFD_EVENT_PAGEFAULT event: Pagefault { kind: Missing, rw: Read, addr: 0x7fd14a781000 }
<pid:20295>        (uffdio_copy.copy returned 4096)
<pid:20297> read address 0x7fd14a78100f: 'E'
<pid:20297> read address 0x7fd14a78140f: 'E'
<pid:20297> read address 0x7fd14a78180f: 'E'
<pid:20297> read address 0x7fd14a781c0f: 'E'
<pid:20295>    UFFD_EVENT_PAGEFAULT event: Pagefault { kind: Missing, rw: Read, addr: 0x7fd14a782000 }
<pid:20295>        (uffdio_copy.copy returned 4096)
<pid:20297> read address 0x7fd14a78200f: 'F'
<pid:20297> read address 0x7fd14a78240f: 'F'
<pid:20297> read address 0x7fd14a78280f: 'F'
<pid:20297> read address 0x7fd14a782c0f: 'F'
<pid:20295>    UFFD_EVENT_PAGEFAULT event: Pagefault { kind: Missing, rw: Read, addr: 0x7fd14a783000 }
<pid:20295>        (uffdio_copy.copy returned 4096)
<pid:20297> read address 0x7fd14a78300f: 'G'
<pid:20297> read address 0x7fd14a78340f: 'G'
<pid:20297> read address 0x7fd14a78380f: 'G'
<pid:20297> read address 0x7fd14a783c0f: 'G'
<pid:20295>    UFFD_EVENT_PAGEFAULT event: Pagefault { kind: Missing, rw: Read, addr: 0x7fd14a784000 }
<pid:20295>        (uffdio_copy.copy returned 4096)
<pid:20297> read address 0x7fd14a78400f: 'H'
<pid:20297> read address 0x7fd14a78440f: 'H'
<pid:20297> read address 0x7fd14a78480f: 'H'
<pid:20297> read address 0x7fd14a784c0f: 'H'
<pid:20295>    UFFD_EVENT_PAGEFAULT event: Pagefault { kind: Missing, rw: Read, addr: 0x7fd14a785000 }
<pid:20295>        (uffdio_copy.copy returned 4096)
<pid:20297> read address 0x7fd14a78500f: 'I'
<pid:20297> read address 0x7fd14a78540f: 'I'
<pid:20297> read address 0x7fd14a78580f: 'I'
<pid:20297> read address 0x7fd14a785c0f: 'I'
<pid:20295>    UFFD_EVENT_PAGEFAULT event: Pagefault { kind: Missing, rw: Read, addr: 0x7fd14a786000 }
<pid:20295>        (uffdio_copy.copy returned 4096)
<pid:20297> read address 0x7fd14a78600f: 'J'
<pid:20297> read address 0x7fd14a78640f: 'J'
<pid:20297> read address 0x7fd14a78680f: 'J'
<pid:20297> read address 0x7fd14a786c0f: 'J'
<pid:20297> child is exiting...
<pid:20295> got signal SIGCHLD from child 20297
<pid:20295> parent is exiting...
```

### Explanation

The program is run with 10 pages as the argument specified. The child process accesses the memory region with a 0x400 increase, triggering page faults when crossing the page boundary. The parent process handles the page faults by providing copies of the data from the specified locations. The child process terminates after accessing all the pages. The parent process exits after receiving the `SIGCHLD` signal from the child process.

The child process creates a memory mapping and reports the starting address. The parent process handles page faults triggered by the child process. Each line beginning with `<pid:20295> UFFD_EVENT_PAGEFAULT event:` indicates a page fault handled by the parent. The address `addr` shows where the fault occurred. The parent process varies the data ('A', 'B', 'C', etc.) it copies into the faulting region on each fault, as indicated by the changing letters in the child's output.

When the child exits, the parent process receives a SIGCHLD signal, indicating that the child process (PID 20297) has exited. The parent then exits as well.

## Future Works

1. Support more than one child process
2. Support switching between child processes and threads
3. Support switching between different memory management strategies
4. Support other memory allocation strategies than the default `mmap` strategy

## Conclusion

This project combines several advanced Linux programming concepts and techniques. It showcases the potential of `userfaultfd` for complex memory management tasks in user space and the effective use of `signalfd` for signal handling. Combining these techniques, developers will be able to create robust solutions for effectively handling the system resources and inter-process synchronization.

## Acknowledgments

This project owes its success to the invaluable resources provided by the `userfaultfd-rs` [Rust crate](https://crates.io/crates/userfaultfd) and the detailed examples featured in the `userfaultfd` system call [man page](https://man7.org/linux/man-pages/man2/userfaultfd.2.html). The implementation of my user space paging program extensively leveraged these examples, adapting and extending them to suit the specific needs of my application.

I extend my sincere thanks to the developers and contributors of the `userfaultfd` Rust crate for their diligent work in creating a robust and user-friendly interface for userfaultfd functionality in Rust. Their commitment to providing a high-quality crate has significantly streamlined my development process, enabling me to efficiently implement advanced memory management features.

Furthermore, I express my gratitude to the authors and maintainers of the `userfaultfd` system call man page. The comprehensive and well-structured examples provided in the man page served as an essential guide and a solid foundation for understanding the intricacies of the userfaultfd system call. These examples were instrumental in helping me grasp the complex concepts involved in user space paging and in implementing the core functionalities of my program.

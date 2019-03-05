use core::fmt;
use osmium_syscall::number;

fn syscall_0(num: u32) -> u32 {
    let result: u32;
    unsafe {
        asm!("
            ecall
        "
        : "={x10}"(result)
        : "{x10}"(num)
        );
    }
    result
}

fn syscall_1(num: u32, a: u32) -> u32 {
    let result: u32;
    unsafe {
        asm!("
            ecall
        "
        : "={x10}"(result)
        : "{x10}"(num), "{x11}"(a)
        );
    }
    result
}

fn syscall_2(num: u32, a: u32, b: u32) -> u32 {
    let result: u32;
    unsafe {
        asm!("
            ecall
        "
        : "={x10}"(result)
        : "{x10}"(num), "{x11}"(a), "{x12}"(b)
        );
    }
    result
}

fn syscall_3(num: u32, a: u32, b: u32, c: u32) -> u32 {
    let result: u32;
    unsafe {
        asm!("
            ecall
        "
        : "={x10}"(result)
        : "{x10}"(num), "{x11}"(a), "{x12}"(b), "{x13}"(c)
        );
    }
    result
}

fn syscall_4(num: u32, a: u32, b: u32, c: u32, d: u32) -> u32 {
    let result: u32;
    unsafe {
        asm!("
            ecall
        "
        : "={x10}"(result)
        : "{x10}"(num), "{x11}"(a), "{x12}"(b), "{x13}"(c), "{x14}"(d)
        );
    }
    result
}

#[derive(Debug, Copy, Clone)]
pub enum SyscallError {
    InvalidSyscallNumber,
    InternalError,
    TooManyProcess,
    NoMemorySpace,
    InvalidArguments,
    IllegalFile,
    NotFound,
}

impl SyscallError {
    pub fn from_syscall_result(x: i32) -> SyscallError {
        match x {
            -1 => SyscallError::InvalidSyscallNumber,
            -2 => SyscallError::InternalError,
            -3 => SyscallError::TooManyProcess,
            -4 => SyscallError::NoMemorySpace,
            -5 => SyscallError::InvalidArguments,
            -6 => SyscallError::IllegalFile,

            -7 => SyscallError::NotFound,
            _ => panic!("illegal syscall result"),
        }
    }
}

impl fmt::Display for SyscallError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

impl SyscallError {
    pub fn to_str(&self) -> &'static str {
        match self {
            SyscallError::InvalidSyscallNumber => "Invalid Syscall Number",
            SyscallError::InternalError => "Internal error",
            SyscallError::TooManyProcess => "Too many process",
            SyscallError::NoMemorySpace => "No Memory Space",
            SyscallError::InvalidArguments => "Invalid Arguments",
            SyscallError::IllegalFile => "Illegal File",
            SyscallError::NotFound => "Not Found",
        }
    }
}

pub fn sys_write(buf: &[u8], size: usize) -> u32 {
    syscall_2(number::SYS_UART_WRITE, buf.as_ptr() as u32, size as u32)
}

pub fn sys_read(buf: &mut [u8], size: usize) -> u32 {
    syscall_2(number::SYS_UART_READ, buf.as_ptr() as u32, size as u32)
}

pub fn sys_exit(status: u32) -> ! {
    syscall_1(number::SYS_EXIT, status);
    loop {} // here does not reach
}

pub fn sys_get_proc_id() -> u32 {
    syscall_0(number::SYS_GET_PROC_ID)
}

pub fn sys_yield() -> u32 {
    syscall_0(number::SYS_YIELD)
}

pub enum ForkResult {
    Parent(u32),
    Child,
    Fail,
}
pub fn sys_fork() -> ForkResult {
    let r = syscall_0(number::SYS_FORK) as i32;
    if r < 0 {
        ForkResult::Fail
    } else if r == 0 {
        ForkResult::Child
    } else {
        ForkResult::Parent(r as u32)
    }
}

pub fn sys_execve(
    filename: &str,
    filename_len: u32,
    argv: &[*const u32],
    envp: &[*const u32],
) -> ! {
    let x = syscall_4(
        number::SYS_EXECVE,
        filename.as_bytes().as_ptr() as u32,
        filename_len,
        argv.as_ptr() as u32,
        envp.as_ptr() as u32,
    );
    if (x as i32) < 0 {
        println!("{}", SyscallError::from_syscall_result(x as i32));
        sys_exit(x)
    }

    loop {}
}

pub enum ProcessStatus {
    Free,
    Running,
    Runnable,
    NotRunnable,
    Zonmbie,
}

impl ProcessStatus {
    pub fn from_u32(x: u32) -> ProcessStatus {
        match x {
            0 => ProcessStatus::Free,
            1 => ProcessStatus::Running,
            2 => ProcessStatus::Runnable,
            3 => ProcessStatus::NotRunnable,
            4 => ProcessStatus::Zonmbie,
            _ => panic!("failed to handle process status"),
        }
    }
}

pub fn sys_check_process_status(id: u32) -> ProcessStatus {
    let r = syscall_1(number::SYS_PROC_STATUS, id);
    ProcessStatus::from_u32(r)
}

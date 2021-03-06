use bounded_buffer as bb;
use core::fmt;
use csr::CSRRead;
use elf;
use memlayout;
use memutil;
use paging;
use satp;
use trap;
use utils;

pub const N_PROCS: usize = 1024;

pub enum Type {
    User,
}

pub enum Status {
    Free,
    Running,
    Runnable,
    NotRunnable,
    Zonmbie,
}

impl Status {
    pub fn to_u32(&self) -> u32 {
        match self {
            Status::Free => 0,
            Status::Running => 1,
            Status::Runnable => 2,
            Status::NotRunnable => 3,
            Status::Zonmbie => 4,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum ProcessError {
    FailedToCreateProcess,
    FailedToMap(paging::PageError),
    ProgramError(&'static str),
    NoSuchProcess,
    QueueIsFull,
    QueueIsEmpty,
}

impl ProcessError {
    fn to_str(&self) -> &'static str {
        match self {
            ProcessError::FailedToCreateProcess => "failed to create process",
            ProcessError::FailedToMap(_) => "failed to map",
            ProcessError::ProgramError(s) => s,
            ProcessError::NoSuchProcess => "no such process",
            ProcessError::QueueIsEmpty => "queue is empty",
            ProcessError::QueueIsFull => "queue is full",
        }
    }
}

impl fmt::Display for ProcessError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ProcessError({})", self.to_str())
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Id(pub u32);

impl Id {
    pub fn to_u32(self) -> u32 {
        self.0
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Message {
    pub id: Id,
    pub data: u32,
}

#[repr(C)]
pub struct Process<'a> {
    pub mapper: paging::Map<'a>,
    pub id: Id,
    pub parent_id: Id,
    index: usize,
    pub proc_type: Type,
    pub status: Status,
    pub trap_frame: trap::TrapFrame,
    pub exit_code: u32,
    message_queue: bb::BoundedBuffer<Message>,
}

impl<'a> Process<'a> {
    pub fn init(&mut self, id: Id, mapper: paging::Map<'a>) {
        self.mapper = mapper;
        self.id = id;
        self.parent_id = id;
        self.proc_type = Type::User;
        self.status = Status::Free;
        self.trap_frame = trap::TrapFrame::new(0, 0);
        self.message_queue = bb::BoundedBuffer::new(Message { id, data: 0 });
    }
    // dont touch without ProcessManager
    pub unsafe fn set_index(&mut self, index: usize) {
        self.index = index;
    }

    pub fn size_of() -> usize {
        use core::intrinsics::size_of_val;
        let dummy = 100usize;
        let p: &Process = unsafe { &*(dummy as *const Process) };
        unsafe { size_of_val(p) }
    }

    pub fn create(&mut self, mapper: &mut paging::Map) -> Result<(), ProcessError> {
        mapper.clone_dir(&mut self.mapper);
        Ok(())
    }

    pub fn ppn(&self) -> u32 {
        self.mapper.ppn()
    }

    pub fn set_trap_frame(&mut self, tf: trap::TrapFrame) {
        self.trap_frame = tf;
    }

    pub fn region_alloc(
        &mut self,
        va: paging::VirtAddr,
        size: usize,
        flag: paging::Flag,
        allocator: &mut paging::Allocator,
    ) -> Result<(), ProcessError> {
        let old_satp = satp::SATP::read_csr();
        satp::SATP::set_ppn(self.ppn());
        for page in paging::Page::range(va, size as u32) {
            match allocator.alloc() {
                Ok(frame) => {
                    dprintln!("{:?} -> {:?}", page, frame);
                    match self.mapper.map(page, frame, flag, allocator) {
                        Ok(_) => (),
                        Err(e) => return Err(ProcessError::FailedToMap(e)),
                    };
                }
                Err(e) => return Err(ProcessError::FailedToMap(e)),
            }
        }
        satp::SATP::set_ppn(old_satp);
        Ok(())
    }
    pub fn load_elf(
        &mut self,
        elf_file: &elf::Elf,
        allocator: &mut paging::Allocator,
    ) -> Result<(), ProcessError> {
        let old_satp = satp::SATP::read_csr();
        satp::SATP::set_ppn(self.ppn());

        for program in elf_file.programs() {
            self.region_alloc(
                program.virt_addr,
                utils::round_up(program.mem_size as u64, paging::PGSIZE as u64) as usize,
                paging::Flag::EXEC
                    | paging::Flag::USER
                    | paging::Flag::READ
                    | paging::Flag::WRITE
                    | paging::Flag::VALID,
                allocator,
            )?;
            let region = program.virt_addr.as_mut_ptr();
            unsafe {
                memutil::memset(
                    region,
                    0,
                    utils::round_up(program.mem_size as u64, paging::PGSIZE as u64) as usize,
                );
                memutil::memcpy(region, program.data, program.file_size);
            }
        }

        // alloc stack
        self.region_alloc(
            paging::VirtAddr::new(memlayout::USER_STACK_TOP),
            memlayout::USER_STACK_SIZE as usize,
            paging::Flag::VALID | paging::Flag::READ | paging::Flag::WRITE | paging::Flag::USER,
            allocator,
        )?;

        satp::SATP::set_ppn(old_satp);
        Ok(())
    }

    pub fn run(&mut self) -> ! {
        dprintln!("I will run: {:x}, {:x}", self.id.0, self.trap_frame.pc);
        satp::SATP::set_ppn(self.ppn());
        self.status = Status::Running;
        unsafe {
            trap::pop_trap_frame(&self.trap_frame);
        }
    }

    pub fn exit(&mut self, exit_code: u32) {
        self.status = Status::Zonmbie;
        self.exit_code = exit_code;
    }

    pub fn enqueue_message(&mut self, id: Id, data: u32) -> Result<(), ProcessError> {
        match self.message_queue.enqueue(Message { id, data }) {
            Ok(()) => Ok(()),
            Err(bb::Error::Full) => Err(ProcessError::QueueIsFull),
            Err(bb::Error::Empty) => Err(ProcessError::QueueIsEmpty),
        }
    }

    pub fn dequeue_message(&mut self) -> Result<Message, ProcessError> {
        match self.message_queue.dequeue() {
            Ok(x) => Ok(x),
            Err(bb::Error::Full) => Err(ProcessError::QueueIsFull),
            Err(bb::Error::Empty) => Err(ProcessError::QueueIsEmpty),
        }
    }
}

pub struct ProcessManager<'a> {
    procs: &'a mut [Process<'a>; N_PROCS],
    id_stack: [usize; N_PROCS],
    stack: usize,
    sched_index: usize,
}

impl<'a> ProcessManager<'a> {
    pub fn new(
        procs: &'a mut [Process<'a>; N_PROCS],
        proc_pages: &'a mut [paging::PageTable; N_PROCS],
        proc_tmp_pages: &'a mut [paging::PageTable; N_PROCS],
    ) -> ProcessManager<'a> {
        let mut id_stack = [0usize; N_PROCS];
        for (i, (p, t)) in proc_pages
            .iter_mut()
            .zip(proc_tmp_pages.iter_mut())
            .enumerate()
        {
            id_stack[i] = i;
            paging::PageTable::setup_tmp_table(p, t);
            procs[i].init(Id(i as u32), paging::Map::new(p, t));
            unsafe { procs[i].set_index(i) };
        }
        ProcessManager {
            procs,
            id_stack,
            stack: N_PROCS,
            sched_index: 0,
        }
    }

    pub fn id2proc(&mut self, id: Id) -> Result<&'a mut Process<'a>, ProcessError> {
        for i in 0..N_PROCS {
            if self.procs[i].id == id {
                let ptr = (&mut self.procs[id.0 as usize]) as *mut Process<'a>;
                let p = unsafe { &mut *ptr };
                return Ok(p);
            }
        }
        Err(ProcessError::NoSuchProcess)
    }
    pub unsafe fn alloc(&mut self) -> Result<*mut Process<'a>, ProcessError> {
        if self.stack == 0 {
            Err(ProcessError::FailedToCreateProcess)
        } else {
            self.stack -= 1;
            let id = self.id_stack[self.stack];
            Ok((&mut self.procs[id]) as *mut Process<'a>)
        }
    }

    pub fn dealloc(&mut self, proc: &Process) -> Result<(), ProcessError> {
        if self.stack == N_PROCS {
            Err(ProcessError::ProgramError("frame stack overflow"))
        } else {
            self.id_stack[self.stack] = proc.index;
            self.stack += 1;
            Ok(())
        }
    }

    pub fn sched(&mut self) -> Option<*mut Process<'a>> {
        let old_index = self.sched_index;
        for i in old_index..N_PROCS {
            match self.procs[i].status {
                Status::Runnable => {
                    self.sched_index = (i + 1) % N_PROCS;
                    return Some((&mut self.procs[i]) as *mut Process<'a>);
                }
                _ => (),
            }
        }
        for i in 0..old_index {
            match self.procs[i].status {
                Status::Runnable => {
                    self.sched_index = (i + 1) % N_PROCS;
                    return Some((&mut self.procs[i]) as *mut Process<'a>);
                }
                _ => (),
            }
        }
        None
    }
}

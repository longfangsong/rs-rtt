use alloc::collections::VecDeque;
use alloc::sync::Arc;
use core::alloc::{GlobalAlloc, Layout};
use core::borrow::Borrow;
use core::borrow::BorrowMut;
use core::cell::RefCell;
use core::fmt::{Debug, Error, Formatter};
use core::ops::{Deref, DerefMut};
use core::{mem, ptr};
use cortex_m::asm::wfi;
use cortex_m::peripheral::scb::SystemHandler;
use cortex_m::register::control::Spsel;
use cortex_m::register::primask;
use cortex_m::{asm, interrupt, register, Peripherals};
use cortex_m_rt::exception;
use cortex_m_rt::ExceptionFrame;
use cortex_m_semihosting::hprintln;
use spin::Mutex;

type Address = u32;
type ThreadEntryFunction = fn(Address) -> ();

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct StackFrame {
    r4: u32,
    r5: u32,
    r6: u32,
    r7: u32,
    r8: u32,
    r9: u32,
    r10: u32,
    r11: u32,
    exception_stack_frame: ExceptionFrame,
}

impl StackFrame {
    fn new(entry: ThreadEntryFunction, param: Address) -> Self {
        StackFrame {
            r4: 0xdeadbeef,
            r5: 0xdeadbeef,
            r6: 0xdeadbeef,
            r7: 0xdeadbeef,
            r8: 0xdeadbeef,
            r9: 0xdeadbeef,
            r10: 0xdeadbeef,
            r11: 0xdeadbeef,
            exception_stack_frame: ExceptionFrame {
                r0: param as _,
                r1: 0,
                r2: 0,
                r3: 0,
                r12: 0,
                lr: 0,
                pc: entry as *const () as _,
                xpsr: 0x01000000,
            },
        }
    }
}

pub struct SimpleThread {
    stack_pointer: Address,
    entry: ThreadEntryFunction,
    param: Address,
    stack: Address,
}

impl Debug for SimpleThread {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(
            f,
            "Thread which sp:0x{:x}, stack:0x{:x}",
            self.stack_pointer, self.stack
        )
    }
}

impl SimpleThread {
    fn new(entry: ThreadEntryFunction, param: Address, stack_size: usize) -> Self {
        unsafe {
            use crate::memory::ALLOCATOR;
            let stack = ALLOCATOR.alloc(Layout::from_size_align(stack_size, 8).unwrap());
            let frame: *mut StackFrame =
                stack.offset((stack_size - mem::size_of::<StackFrame>()) as isize) as _;
            *frame = StackFrame::new(entry, param);
            SimpleThread {
                stack_pointer: frame as _,
                entry,
                param,
                stack: stack as _,
            }
        }
    }
}

lazy_static! {
    pub static ref PENDING_LIST: Mutex<RefCell<VecDeque<Arc<SimpleThread>>>> =
        Mutex::new(RefCell::new(VecDeque::new()));
}

fn schedule() {
    if PENDING_LIST.lock().deref().borrow().len() >= 2 {
        let to_schedule_out = PENDING_LIST
            .lock()
            .deref()
            .borrow_mut()
            .pop_front()
            .unwrap();
        let to_schedule_in = PENDING_LIST.lock().deref().borrow().front();
        PENDING_LIST
            .lock()
            .deref()
            .borrow_mut()
            .push_back(to_schedule_out);
    }
}

fn idle(_: Address) {
    loop {
        asm::nop();
    }
}

struct ContextSwitchService {
    switch_from_sp: u32,
    switch_to_sp: u32,
}

impl ContextSwitchService {
    fn request_switch(&mut self) {
        if self.switch_to_sp != 0 && self.switch_to_sp != self.switch_from_sp {
            cortex_m::peripheral::SCB::set_pendsv();
        }
    }
}

static mut CONTEXT_SWITCH_SERVICE: ContextSwitchService = ContextSwitchService {
    switch_from_sp: 0,
    switch_to_sp: 0,
};

#[naked]
#[no_mangle]
#[inline(never)]
unsafe extern "C" fn PendSV() {
    asm!("mrs r3, PRIMASK
          cpsid i":::"r3":"volatile");
    // force the compiler generate the code which load
    // switch_from_sp and switch_to_sp
    // inside the critical area
    asm!("cbz $0, switch_to
          switch_from:
            mrs $0, psp
            stmfd $0!, {r4-r11}
          switch_to:
            ldmfd $1!, {r4-r11}
            msr psp, $1
          msr PRIMASK, r3
          orr lr, lr, #0x04"
          : "=r"(CONTEXT_SWITCH_SERVICE.switch_from_sp), "=r"(CONTEXT_SWITCH_SERVICE.switch_to_sp)
          : "r"(CONTEXT_SWITCH_SERVICE.switch_from_sp), "r"(CONTEXT_SWITCH_SERVICE.switch_to_sp)
          : "r3", "r4", "r5", "r6", "r7", "r8", "r9", "r10", "r11", "lr", "psp", "memory"
          : "volatile");
    // bx lr is auto added
}

pub fn init() {
    unsafe {
        let mut peripherals = Peripherals::steal();
        peripherals.SCB.set_priority(SystemHandler::PendSV, 0xff);
    }
    PENDING_LIST
        .lock()
        .deref()
        .borrow_mut()
        .push_back(Arc::new(SimpleThread::new(idle, 0, 64)));
    unsafe {
        CONTEXT_SWITCH_SERVICE.switch_to_sp = PENDING_LIST
            .lock()
            .deref()
            .borrow()
            .front()
            .unwrap()
            .stack_pointer;
        CONTEXT_SWITCH_SERVICE.request_switch();
    }
}

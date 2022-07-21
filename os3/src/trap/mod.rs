use core::arch::asm;

use crate::{syscall::syscall, task::get_task_manager, timer::set_next_trigger};
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Interrupt, Trap},
    sie,
    sstatus::{self, Sstatus, SPP},
    stval, stvec,
};

core::arch::global_asm!(include_str!("trap.S"));

#[repr(C)]
pub struct TrapContext {
    pub regs: [usize; 32],
    pub sstatus: Sstatus,
    pub sepc: usize,
}

impl TrapContext {
    pub fn set_sp(&mut self, sp: usize) {
        self.regs[2] = sp;
    }

    pub fn app_init_context(entry: usize, sp: usize) -> Self {
        let sstatus = sstatus::read();
        unsafe {
            sstatus::set_spp(SPP::User);
        };
        let mut context = Self {
            regs: [0; 32],
            sstatus,
            sepc: entry,
        };

        context.set_sp(sp);
        context
    }
}

pub fn init() {
    extern "C" {
        fn __alltraps();
    }
    unsafe {
        stvec::write(__alltraps as usize, TrapMode::Direct);
    }
}

pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}

#[no_mangle]
pub fn trap_handler(ctx: &mut TrapContext) -> &mut TrapContext {
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            ctx.sepc += 4;
            ctx.regs[10] = syscall(
                ctx.regs[17],
                [ctx.regs[10], ctx.regs[11], ctx.regs[12]],
                ctx as *const _ as usize,
            ) as usize;
        }
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::StorePageFault) => {
            error!("[kernel] PageFault in application, bad addr = {:#x}, bad instruction = {:#x}, core dumped.", stval, ctx.sepc);
            get_task_manager().exit_and_run_next_app();
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            error!("[kernel] IllegalInstruction in application, core dumped.");
            get_task_manager().exit_and_run_next_app();
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            set_next_trigger();
            get_task_manager().suspend_and_run_next_app(ctx as *const _ as usize);
            error!("[kernel] time triggerred");
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
    ctx
}

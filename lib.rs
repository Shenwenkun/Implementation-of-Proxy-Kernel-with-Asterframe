#![no_std]

pub mod scheduler;

extern crate alloc;

use align_ext::AlignExt;
use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec;
use scheduler::EasyScheduler;
use core::str;
use xmas_elf::ElfFile;

use aster_frame::cpu::UserContext;
use aster_frame::prelude::*;
use aster_frame::task::{set_scheduler, Task, TaskOptions};
use aster_frame::user::{UserEvent, UserMode, UserSpace};
use aster_frame::vm::{Vaddr, VmAllocOptions, VmIo, VmMapOptions, VmPerm, VmSpace, PAGE_SIZE};

static GLOBAL_SCHEDULER: EasyScheduler = EasyScheduler::new();

/// The kernel's boot and initialization process is managed by Asterinas Framework.
/// After the process is done, the kernel's execution environment
/// (e.g., stack, heap, tasks) will be ready for use and the entry function
/// labeled as `#[aster_main]` will be called.
#[aster_main]
pub fn main() {
    let program_binary = include_bytes!("../hello_world.bin");
    let file = ElfFile::new(program_binary).unwrap();//read elf
    let user_space = create_user_space(program_binary);
    let user_task = create_user_task(Arc::new(user_space));
    set_scheduler(&GLOBAL_SCHEDULER);
    user_task.run();
}

fn create_user_space(program: &[u8]) -> UserSpace {
    let user_pages = {
        let nframes  = program.len().align_up(PAGE_SIZE) / PAGE_SIZE;
        println!("frames:{:?}",nframes);
        let vm_frames = VmAllocOptions::new(nframes).alloc().unwrap();
        // Phyiscal memory pages can be only accessed
        // via the VmFrame abstraction.
        vm_frames.write_bytes(0, program).unwrap();
        let mut buf = [0,0,0,0,00,0,0,0,0,00,0,0,0,0];
        vm_frames.read_bytes(0x1000, &mut buf);
        println!("test:{:#x?}",buf);
        vm_frames
    };
    let user_address_space = {
        const MAP_ADDR: Vaddr = 0x0040_0000; // The map addr for statically-linked executable

        // The page table of the user space can be
        // created and manipulated safely through
        // the VmSpace abstraction.
        let vm_space = VmSpace::new();
        let mut options = VmMapOptions::new();
        options.addr(Some(MAP_ADDR)).perm(VmPerm::RWX);
        vm_space.map(user_pages, &options).unwrap();
        vm_space
    };
    let user_cpu_state = {
        const ENTRY_POINT: Vaddr = 0x0040_1000; // The entry point for statically-linked executable

        // The user-space CPU states can be initialized
        // to arbitrary values via the UserContext
        // abstraction.
        let mut user_cpu_state = UserContext::default();
        user_cpu_state.set_rip(ENTRY_POINT);
        // FIXME: 
        user_cpu_state.set_rsp(0x40_2000);
        user_cpu_state
    };
    UserSpace::new(user_address_space, user_cpu_state)
}

fn create_user_task(user_space: Arc<UserSpace>) -> Arc<Task> {
    fn user_task() {
        let current = Task::current();
        // Switching between user-kernel space is
        // performed via the UserMode abstraction.
        let mut user_mode = {
            let user_space = current.user_space().unwrap();
            UserMode::new(user_space)
        };

        loop {
            // The execute method returns when system
            // calls or CPU exceptions occur.
            // println!("Running binary");
            let user_event = user_mode.execute();
            // The CPU registers of the user space
            // can be accessed and manipulated via
            // the `UserContext` abstraction.
            let user_context = user_mode.context_mut();
            // println!("complete:{:#x?}",user_context);
            if UserEvent::Syscall == user_event {
                handle_syscall(user_context, current.user_space().unwrap());
            }
        }
    }

    // Kernel tasks are managed by the Framework,
    // while scheduling algorithms for them can be
    // determined by the users of the Framework.
    TaskOptions::new(user_task)
        .user_space(Some(user_space))
        .data(0)
        .build()
        .unwrap()
}

fn handle_syscall(user_context: &mut UserContext, user_space: &UserSpace) {
    const SYS_WRITE: usize = 1;
    const SYS_EXIT: usize = 60;
    println!("Running handle_syscall");

    match user_context.rax() {
        SYS_WRITE => {
            // Access the user-space CPU registers safely.
            let (fd, buf_addr, buf_len) = 
                (user_context.rdi(), user_context.rsi(), user_context.rdx());
            let buf = {
                let mut buf = vec![0u8; buf_len];
                // Copy data from the user space without
                // unsafe pointer dereferencing.
                user_space
                    .vm_space()
                    .read_bytes(buf_addr, &mut buf)
                    .unwrap();
                buf
            };
            // Use the console for output safely.
            println!("{}", str::from_utf8(&buf).unwrap());
            // Manipulate the user-space CPU registers safely.
            user_context.set_rax(buf_len);
        }
        SYS_EXIT => Task::current().exit(),
        _ => unimplemented!(),
    }
}
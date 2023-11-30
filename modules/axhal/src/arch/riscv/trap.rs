use riscv::register::scause::{self, Exception as E, Trap};

use super::TrapFrame;

include_asm_marcos!();

core::arch::global_asm!(
    include_str!("trap.S"),
    trapframe_size = const core::mem::size_of::<TrapFrame>(),
);

fn handle_breakpoint(sepc: &mut usize) {
    debug!("Exception(Breakpoint) @ {:#x} ", sepc);
    *sepc += 2
}

fn dump_instructions_at(base: usize, from: isize, to: isize) {
    for offset in from..=to {
        let addr: usize = base.wrapping_add_signed(offset * 4);
        let zeroMark = if offset == 0 { '*' } else { ' ' };

        debug!("{}{:#016x}: {:032b}", zeroMark, addr, unsafe {
            *(addr as *const u32)
        })
    }
}

#[no_mangle]
fn riscv_trap_handler(tf: &mut TrapFrame, _from_user: bool) {
    let scause = scause::read();
    match scause.cause() {
        Trap::Exception(E::Breakpoint) => handle_breakpoint(&mut tf.sepc),
        Trap::Interrupt(_) => crate::trap::handle_irq_extern(scause.bits()),
        Trap::Exception(E::IllegalInstruction) => {
            dump_instructions_at(tf.sepc, -4, 4);
            panic!("Illegal instruction @ {:#x}:\n{:#x?}", tf.sepc, tf);
        }
        _ => {
            let vsatp: usize;
            let hgatp: usize;
            let stval: usize;
            unsafe {
                core::arch::asm!("
                    csrr {}, vsatp
                    csrr {}, hgatp
                    csrr {}, stval
                    ",
                    out(reg) vsatp,
                    out(reg) hgatp,
                    out(reg) stval,
                    options(nostack, nomem)
                );
            }
            warn!("vsatp = {vsatp:#x}, hgatp = {hgatp:#x}, stval= {stval:#x}");
            panic!(
                "Unhandled trap {:?} @ {:#x}:\n{:#x?}",
                scause.cause(),
                tf.sepc,
                tf
            );
        }
    }
}

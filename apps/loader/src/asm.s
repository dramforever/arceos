    .macro func name
        .type \name, @function
    \name:
    .endm

    .macro endfunc name
        .size \name, . - \name
    .endm

    // fn enter_program(sp: usize, entry: usize) -> !;
    .global enter_program
func enter_program
    mv sp, a0
    lla a0, axmusl_syscall_handler_wrap
    jalr a1
endfunc enter_program


func axmusl_syscall_handler_wrap
    addi sp, sp, -32
    sd ra, 0(sp)
    sd gp, 8(sp)
    sd tp, 16(sp)

    ld gp, 0(a0)
    ld tp, 8(a0)
    call axmusl_syscall_handler

    ld ra, 0(sp)
    ld gp, 8(sp)
    ld tp, 16(sp)
    addi sp, sp, 32
    ret
endfunc axmusl_syscall_handler_wrap

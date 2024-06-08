use aya_ebpf::cty::c_int;
use aya_ebpf::macros::lsm;
use aya_ebpf::programs::LsmContext;
use aya_log_ebpf::info;

use crate::vmlinux::generated::task_struct;

#[lsm(hook = "task_alloc")]
pub fn task_alloc(ctx: LsmContext) -> i32 {
    unsafe { try_task_alloc(ctx).unwrap_or_else(|ret| ret) }
}

unsafe fn try_task_alloc(ctx: LsmContext) -> Result<i32, i32> {
    let task: *const task_struct = ctx.arg(0);
    let pid: c_int = (*task).pid;
    info!(&ctx, "Process with PID {} spawned a child process", pid);
    Ok(0)
}

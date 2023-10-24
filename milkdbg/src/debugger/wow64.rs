pub fn get_thread_context(
    handle: winapi::um::winnt::HANDLE,
) -> Result<winapi::um::winnt::WOW64_CONTEXT, u32> {
    unsafe {
        let mut ctx: winapi::um::winnt::WOW64_CONTEXT = Default::default();
        ctx.ContextFlags = winapi::um::winnt::WOW64_CONTEXT_ALL;
        let r = winapi::um::winbase::Wow64GetThreadContext(handle, &mut ctx);
        if r != 0 {
            Ok(ctx)
        } else {
            Err(winapi::um::errhandlingapi::GetLastError())
        }
    }
}

pub fn set_thread_context(
    handle: winapi::um::winnt::HANDLE,
    ctx: winapi::um::winnt::WOW64_CONTEXT,
) -> Result<(), u32> {
    unsafe {
        let r = winapi::um::winbase::Wow64SetThreadContext(handle, &ctx);
        if r != 0 {
            Ok(())
        } else {
            Err(winapi::um::errhandlingapi::GetLastError())
        }
    }
}

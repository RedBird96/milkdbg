//http://msdn.microsoft.com/en-us/library/windows/desktop/ms679295%28v=vs.85%29.aspx
pub fn debug_active_process(pid: usize) -> Result<(), u32> {
    unsafe {
        let r = winapi::um::debugapi::DebugActiveProcess(pid as u32);
        if r != 0 {
            Ok(())
        } else {
            Err(winapi::um::errhandlingapi::GetLastError())
        }
    }
}

//https://docs.microsoft.com/en-us/windows/win32/api/debugapi/nf-debugapi-debugactiveprocessstop
#[allow(dead_code)]
pub fn debug_active_process_stop(pid: usize) -> Result<(), u32> {
    unsafe {
        let r = winapi::um::debugapi::DebugActiveProcessStop(pid as u32);
        if r != 0 {
            Ok(())
        } else {
            Err(winapi::um::errhandlingapi::GetLastError())
        }
    }
}

pub fn read_process_memory(
    process: winapi::um::winnt::HANDLE,
    addr: usize,
    size: usize,
) -> Result<Vec<u8>, u32> {
    let mut v = vec![0u8; size];
    let mut read = 0;
    unsafe {
        let r = winapi::um::memoryapi::ReadProcessMemory(
            process,
            addr as *const winapi::ctypes::c_void,
            v.as_mut_ptr() as *mut winapi::ctypes::c_void,
            size,
            &mut read,
        );
        if r != 0 {
            Ok(v)
        } else {
            Err(winapi::um::errhandlingapi::GetLastError())
        }
    }
}

pub fn write_process_memory(
    process: winapi::um::winnt::HANDLE,
    addr: usize,
    data: &[u8],
) -> Result<(), u32> {
    let mut written = 0;
    unsafe {
        let r = winapi::um::memoryapi::WriteProcessMemory(
            process,
            addr as *mut winapi::ctypes::c_void,
            data.as_ptr() as *const winapi::ctypes::c_void as *mut winapi::ctypes::c_void,
            data.len(),
            &mut written,
        );
        if r != 0 {
            Ok(())
        } else {
            Err(winapi::um::errhandlingapi::GetLastError())
        }
    }
}

use bitflags::bitflags;

bitflags! {
    pub struct OpenThreadAccess: u32 {
        const GET_CONTEXT = 00001000;
        const SET_CONTEXT = 00010000;
    }
}

bitflags! {
    pub struct OpenProcessAccess: u32 {
        const PROCESS_QUERY_INFORMATION = 0x0400;
    }
}

pub fn open_thread(
    access: OpenThreadAccess,
    inherit_handle: bool,
    tid: u32,
) -> Result<winapi::um::winnt::HANDLE, u32> {
    unsafe {
        let r = winapi::um::processthreadsapi::OpenThread(
            access.bits(),
            if inherit_handle { 1 } else { 0 },
            tid,
        );
        if r != std::ptr::null_mut() {
            Ok(r)
        } else {
            Err(winapi::um::errhandlingapi::GetLastError())
        }
    }
}

pub fn get_thread_context(
    thread: winapi::um::winnt::HANDLE,
) -> Result<winapi::um::winnt::CONTEXT, u32> {
    unsafe {
        let mut ctx: winapi::um::winnt::CONTEXT = Default::default();
        let r = winapi::um::processthreadsapi::GetThreadContext(thread, &mut ctx);
        if r != 0 {
            Ok(ctx)
        } else {
            Err(winapi::um::errhandlingapi::GetLastError())
        }
    }
}

pub fn set_thread_context(
    handle: winapi::um::winnt::HANDLE,
    ctx: winapi::um::winnt::CONTEXT,
) -> Result<(), u32> {
    unsafe {
        let r = winapi::um::processthreadsapi::SetThreadContext(handle, &ctx);
        if r != 0 {
            Ok(())
        } else {
            Err(winapi::um::errhandlingapi::GetLastError())
        }
    }
}

pub fn is_wow64_process(process: winapi::um::winnt::HANDLE) -> bool {
    unsafe {
        let pid = winapi::um::processthreadsapi::GetProcessId(process);
        let handle = winapi::um::processthreadsapi::OpenProcess(
            OpenProcessAccess::PROCESS_QUERY_INFORMATION.bits(),
            0,
            pid,
        );
        let mut process_arch = 0;
        let mut machine_arch = 0;
        let iswow64 =
            winapi::um::wow64apiset::IsWow64Process2(handle, &mut process_arch, &mut machine_arch);
        winapi::um::handleapi::CloseHandle(handle);
        if iswow64 == 0 {
            false
        } else {
            true
        }
    }
}

bitflags::bitflags! {
    pub struct FileAccess: u32 {
        const GENERIC_READ = 0x80000000;
        const GENERIC_WRITE = 0x40000000;
        const GENERIC_EXECUTE = 0x20000000;
        const GENERIC_ALL = 0x10000000;

        const STANDARD_RIGHTS_READ =  131_072u32;

        const FILE_GENERIC_READ = 1_179_785u32;
        const FILE_GENERIC_WRITE = 1_179_926u32;
    }
}

bitflags::bitflags! {
    pub struct FileShare: u32 {
        const FILE_SHARE_READ = 0x00000001;
        const FILE_SHARE_WRITE = 0x00000002;
        const FILE_SHARE_DELETE = 0x00000004;
    }
}

bitflags::bitflags! {
    pub struct CreationDisposition: u32 {
        const CREATE_NEW = 0x00000001;
        const CREATE_ALWAYS = 0x00000002;
        const OPEN_ALWAYS = 0x00000004;
        const OPEN_EXISTING = 0x00000003;
        const TRUNCATE_EXISTING = 0x00000005;
    }
}

bitflags::bitflags! {
    pub struct FlagsAndAttributes: u32 {
        const FILE_ATTRIBUTE_ARCHIVE = 32;
        const FILE_ATTRIBUTE_ENCRYPTED = 16384;
        const FILE_ATTRIBUTE_HIDDEN = 2;
        const FILE_ATTRIBUTE_NORMAL = 128;
        const FILE_ATTRIBUTE_OFFLINE = 4096;
        const FILE_ATTRIBUTE_READONLY = 1;
        const FILE_ATTRIBUTE_SYSTEM = 4;
        const FILE_ATTRIBUTE_TEMPORARY = 256;
        const FILE_FLAG_BACKUP_SEMANTICS = 0x02000000;
        const FILE_FLAG_DELETE_ON_CLOSE = 0x04000000;
        const FILE_FLAG_NO_BUFFERING = 0x20000000;
        const FILE_FLAG_OPEN_NO_RECALL = 0x00100000;
        const FILE_FLAG_OPEN_REPARSE_POINT = 0x00200000;
        const FILE_FLAG_POSIX_SEMANTICS = 0x01000000;
        const FILE_FLAG_RANDOM_ACCESS = 0x10000000;
        const FILE_FLAG_SESSION_AWARE = 0x00800000;
        const FILE_FLAG_SEQUENTIAL_SCAN = 0x08000000;
        const FILE_FLAG_WRITE_THROUGH = 0x80000000;
    }
}

pub fn get_final_path_name_by_handle(handle: winapi::um::winnt::HANDLE) -> String {
    let mut buffer = vec![0i8; 1024];
    unsafe {
        winapi::um::fileapi::GetFinalPathNameByHandleA(
            handle,
            buffer.as_mut_ptr() as *mut i8,
            buffer.len() as u32,
            0,
        );
        super::helpers::string_from_array_with_zero(buffer.as_slice())
    }
}

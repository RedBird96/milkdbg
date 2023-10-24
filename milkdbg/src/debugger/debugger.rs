use super::helpers::*;
use super::known_api::*;
use super::modules::Modules;
use super::w32::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

use log::debug;
use log::trace;

fn high_u8(v: u64) -> u8 {
    ((v & 0xFF00) >> 8) as u8
}

fn low_u8(v: u64) -> u8 {
    (v & 0xFF) as u8
}

fn low_u16(v: u64) -> u16 {
    (v & 0xFFFF) as u16
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ThreadContext {
    pub ip: u64,
    pub sp: u64,
    pub bp: u64,
    pub ax: u64,
    pub bx: u64,
    pub cx: u64,
    pub dx: u64,
    pub si: u64,
    pub di: u64,
    pub dr6: u64,
}

impl ThreadContext {
    pub fn get(&self, r: iced_x86::Register) -> u64 {
        match r {
            iced_x86::Register::EAX => self.ax,
            iced_x86::Register::EBX => self.bx,
            iced_x86::Register::ECX => self.cx,
            iced_x86::Register::EDX => self.dx,
            iced_x86::Register::ESP => self.sp,
            iced_x86::Register::EBP => self.bp,
            iced_x86::Register::ESI => self.si,
            iced_x86::Register::EIP => self.ip,
            iced_x86::Register::EDI => self.di,

            iced_x86::Register::RAX => self.ax,
            iced_x86::Register::RBX => self.bx,
            iced_x86::Register::RCX => self.cx,
            iced_x86::Register::RDX => self.dx,
            iced_x86::Register::RSP => self.sp,
            iced_x86::Register::RBP => self.bp,
            iced_x86::Register::RSI => self.si,
            iced_x86::Register::RIP => self.ip,
            iced_x86::Register::RDI => self.di,

            iced_x86::Register::AH => high_u8(self.ax) as u64,
            iced_x86::Register::BH => high_u8(self.bx) as u64,
            iced_x86::Register::CH => high_u8(self.cx) as u64,
            iced_x86::Register::DH => high_u8(self.dx) as u64,

            iced_x86::Register::AL => low_u8(self.ax) as u64,
            iced_x86::Register::BL => low_u8(self.bx) as u64,
            iced_x86::Register::CL => low_u8(self.cx) as u64,
            iced_x86::Register::DL => low_u8(self.dx) as u64,

            iced_x86::Register::AX => low_u16(self.ax) as u64,
            iced_x86::Register::BX => low_u16(self.bx) as u64,
            iced_x86::Register::CX => low_u16(self.cx) as u64,
            iced_x86::Register::DX => low_u16(self.dx) as u64,
            iced_x86::Register::SI => low_u16(self.si) as u64,
            iced_x86::Register::DI => low_u16(self.di) as u64,
            iced_x86::Register::BP => low_u16(self.bp) as u64,

            iced_x86::Register::None => 0,
            x @ _ => todo!("{:?}", x),
        }
    }
}

#[derive(Clone)]
pub struct UnresolvedBreakpoint {
    symbol: String,
    slot: usize,
}

impl std::fmt::Debug for UnresolvedBreakpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UnresolvedBreakpoint").field("symbol", &self.symbol).field("slot", &self.slot).finish()
    }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum Breakpoint {
    Simple {
        location: usize,
        original_value: Vec<u8>,
        once: bool,
        trace: bool,
        go: bool
    },
    KnowApi {
        location: usize,
        original_value: Vec<u8>,
        api: KnownApi,
    },
    Unresolved,
}

pub struct Debugger {
    process: winapi::um::winnt::HANDLE,
    pid: usize,
    // tid: usize,
    modules: Modules,
    known_apis: KnownApiDatabase,

    breakpoints_locations: HashMap<usize, usize>,
    pub breakpoints: Vec<Breakpoint>,
    unresolved_breakpoints: Vec<UnresolvedBreakpoint>,
    breakpoint_entrypoint: Option<usize>,
    reactivate_breakpoint: Option<usize>,

    break_on_next_single_step: bool,

    last_debug_event: winapi::um::minwinbase::DEBUG_EVENT,
    current_tid: usize,
    current_known_call: Option<KnownCall>,
}

impl Debugger {
    pub fn new() -> Self {
        Self {
            process: std::ptr::null_mut(),
            pid: 0,

            last_debug_event: winapi::um::minwinbase::DEBUG_EVENT::default(),
            modules: Modules::new(),
            breakpoints_locations: HashMap::new(),
            breakpoints: Vec::new(),
            breakpoint_entrypoint: None,
            unresolved_breakpoints: Vec::new(),
            reactivate_breakpoint: None,
            known_apis: KnownApiDatabase::new(),

            break_on_next_single_step: false,

            current_tid: 0,
            current_known_call: None,
        }
    }

    pub fn get_current_known_call(&self) -> Option<&KnownCall> {
        self.current_known_call.as_ref()
    }

    fn set_cc(&self, location: usize) -> Vec<u8> {
        let opcode = vec![0xcc];

        let original_value = read_process_memory(self.process, location, 1).unwrap();
        write_process_memory(self.process, location, opcode.as_slice()).unwrap();
        original_value
    }

    pub fn add_breakpoint_simple(&mut self, location: usize, once: bool) -> usize {
        let original_value = self.set_cc(location);
        self.breakpoints_locations
            .insert(location, self.breakpoints.len());
        self.breakpoints.push(Breakpoint::Simple {
            location,
            original_value,
            once,
            trace: false,
            go: false
        });
        self.breakpoints.len() - 1
    }

    pub fn add_breakpoint_trace(&mut self, location: usize, once: bool) -> usize {
        // trace!("add_breakpoint_trace: {}", location);

        let original_value = self.set_cc(location);
        self.breakpoints_locations
            .insert(location, self.breakpoints.len());
        self.breakpoints.push(Breakpoint::Simple {
            location,
            original_value,
            once,
            trace: true,
            go: true
        });
        self.breakpoints.len() - 1
    }

    pub fn add_breakpoint_memory(&mut self, location: usize) -> usize {
        let handle = open_thread(
            OpenThreadAccess::GET_CONTEXT | OpenThreadAccess::SET_CONTEXT,
            false,
            self.current_tid as u32,
        )
        .unwrap();

        if is_wow64_process(handle) {
            let mut ctx = super::wow64::get_thread_context(handle).unwrap();
            ctx.Dr0 = location as u32;
            ctx.Dr7 |= 1;
            ctx.Dr7 &= 0xFFF0FFFF;
            ctx.Dr6 = 0;
            let _ = super::wow64::set_thread_context(handle, ctx);
        } else {
            todo!();
        }

        0
    }

    pub fn add_breakpoint_knownapi(&mut self, location: usize, api: KnownApi) {
        let original_value = self.set_cc(location);

        self.breakpoints_locations
            .insert(location, self.breakpoints.len());
        self.breakpoints.push(Breakpoint::KnowApi {
            location,
            original_value,
            api,
        });
    }

    pub fn add_breakpoint_symbol(&mut self, _module: &str, symbol: &str) -> usize {
        debug!("add_breakpoint_symbol: {}", symbol);

        self.breakpoints.push(Breakpoint::Unresolved);
        let slot = self.breakpoints.len() - 1;

        self.unresolved_breakpoints.push(UnresolvedBreakpoint {
            symbol: symbol.to_string(),
            slot,
        });
        self.try_resolve_breakpoints();

        slot
    }

    pub fn start(&mut self, path: &str) {
        debug!(target:"debugger", "path: {}", path);

        let pe = milk_pe_parser::PE::parse(path).unwrap();
        let entry_point = pe.optional.get_address_of_entry_point().to_va(0x400000);

        let path = PathBuf::from_str(path).unwrap();
        let parent = path.parent().unwrap();
        let parent = parent.to_str().unwrap();

        let path = path.to_str().unwrap();
        let mut path = path.to_string();
        path.push('\0');

        let mut parent = parent.to_string();
        parent.push('\0');

        debug!(target:"debugger", "working directory: {:?}", parent);

        unsafe {
            let mut startup_info: winapi::um::processthreadsapi::STARTUPINFOA = Default::default();
            let mut process_info: winapi::um::processthreadsapi::PROCESS_INFORMATION =
                Default::default();
            let _ = winapi::um::processthreadsapi::CreateProcessA(
                path.as_ptr() as *mut i8,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
                winapi::um::winbase::CREATE_SUSPENDED | winapi::um::winbase::DEBUG_PROCESS,
                std::ptr::null_mut(),
                parent.as_ptr() as *mut i8,
                &mut startup_info,
                &mut process_info,
            );

            self.process = process_info.hProcess;
            self.pid = process_info.dwProcessId as usize;
            self.current_tid = process_info.dwThreadId as usize;

            debug!(target:"debugger", "pid: {}", process_info.dwProcessId);
            debug!(target:"debugger", "tid: {}", process_info.dwThreadId);
            debug!(target:"debugger", "entrypoint at: 0x{:X?}", entry_point);
            self.breakpoint_entrypoint = Some(entry_point);

            self.attach(self.pid);
            self.resume_tread(process_info.hThread);
        }
    }

    fn resume_tread(&self, thread: winapi::um::winnt::HANDLE) {
        debug!(target:"debugger", "Resuming Thread: {:?}", thread);
        unsafe {
            winapi::um::processthreadsapi::ResumeThread(thread);
        }
    }

    pub fn attach(&mut self, pid: usize) {
        trace!(target:"debugger", "attach - begin");

        self.pid = pid;
        let _ = debug_active_process(self.pid);

        trace!(target:"debugger", "attach - end");
    }

    fn get_debug_event() -> Result<winapi::um::minwinbase::DEBUG_EVENT, u32> {
        trace!(target:"debugger", "get_debug_event - begin");
        let v = unsafe {
            let mut e: winapi::um::minwinbase::DEBUG_EVENT = Default::default();
            let r = winapi::um::debugapi::WaitForDebugEvent(&mut e, winapi::um::winbase::INFINITE);
            if r != 0 {
                Ok(e)
            } else {
                Err(winapi::um::errhandlingapi::GetLastError())
            }
        };
        trace!(target:"debugger", "get_debug_event - end");
        v
    }

    pub fn continue_debug_event(&self, pid: usize, tid: usize) {
        trace!(target:"debugger", "continue_debug_event - begin");
        unsafe {
            winapi::um::debugapi::ContinueDebugEvent(
                pid as u32,
                tid as u32,
                winapi::shared::ntstatus::DBG_CONTINUE as u32,
            );
        }
        trace!(target:"debugger", "continue_debug_event - end");
    }

    pub fn step(&mut self) {
        let _ = self.turnon_single_step(self.current_tid as u32, None);
        self.break_on_next_single_step = true;
        self.go();
    }

    pub fn go(&mut self) {
        trace!(target:"debugger", "go - begin");

        self.current_known_call = None;

        loop {
            if self.last_debug_event.dwProcessId != 0 {
                self.continue_debug_event(
                    self.last_debug_event.dwProcessId as usize,
                    self.last_debug_event.dwThreadId as usize,
                );
            }

            let e = Self::get_debug_event();
            match e {
                Ok(e) => {
                    self.last_debug_event = e;
                    self.current_tid = self.last_debug_event.dwThreadId as usize;

                    use winapi::um::minwinbase::*;
                    match e.dwDebugEventCode {
                        CREATE_PROCESS_DEBUG_EVENT => {
                            let info = unsafe { e.u.CreateProcessInfo() };
                            self.process = info.hProcess;
                            // TODO: get pid
                            // TODO: get tid

                            let mut module_name =
                                read_string_char_by_char(info.hProcess, info.lpImageName as usize)
                                    .unwrap();
                            let path = PathBuf::from(get_final_path_name_by_handle(info.hFile));
                            if module_name.len() == 0 {
                                module_name =
                                    path.file_name().unwrap().to_str().unwrap().to_string();
                            }
                            let size = std::fs::metadata(&path).unwrap().len();

                            debug!(target:"debugger", "Process: {} at {:?}", module_name, path);

                            self.modules.process = Some(info.hProcess);
                            let _ = self.modules.load_module(
                                info.lpBaseOfImage as usize,
                                size as usize,
                                module_name.as_str(),
                            );
                            self.try_resolve_breakpoints();

                            // if let Some(entry_point) = info.lpStartAddress {
                            //     self.add_breakpoint_once(entry_point as usize);
                            //     println!("{:?}", self.breakpoints.last());
                            // }
                        }
                        CREATE_THREAD_DEBUG_EVENT => {}
                        EXCEPTION_DEBUG_EVENT => {
                            let info = unsafe { e.u.Exception() };

                            let code = info.ExceptionRecord.ExceptionCode;
                            let addr = info.ExceptionRecord.ExceptionAddress as usize;

                            match code {
                                EXCEPTION_ACCESS_VIOLATION => {
                                    panic!();
                                }
                                EXCEPTION_ARRAY_BOUNDS_EXCEEDED => {
                                    println!("\tEXCEPTION_ARRAY_BOUNDS_EXCEEDED")
                                }
                                EXCEPTION_BREAKPOINT | 1073741855 => {
                                    // Check we care about this breakpoint
                                    if let Some((i, b)) = self
                                        .breakpoints_locations
                                        .get(&addr)
                                        .and_then(|i| self.breakpoints.get(*i).map(|b| (*i, b)))
                                    {
                                        debug!(target:"debugger", "Breakpoint hit at 0x{:08X}", addr);
                                        self.restore_original(b);
                                        let _ = self.turnon_single_step(
                                            self.current_tid as u32,
                                            Some(addr as u64),
                                        );
                                        self.reactivate_breakpoint = Some(i);
                                        match b {
                                            Breakpoint::KnowApi { api, .. } => {
                                                let call = api.parse_know_call(
                                                    self.process,
                                                    self.current_tid as u32,
                                                );
                                                debug!(target:"debugger", "Know Call: {:?}", call);
                                                self.current_known_call = Some(call);
                                            }
                                            Breakpoint::Simple { trace, ..} if *trace=> {
                                                let s = if let Some((addr, i)) = self.get_current_instruction() {
                                                    let s = self.format_instruction(i);
                                                    format!("0x{:X} {}", addr, s)
                                                } else {
                                                    format!("<ERROR>")
                                                };
                                                println!("{}", s);
                                            }
                                            _ => {}
                                        };

                                        break;
                                    }
                                }
                                EXCEPTION_DATATYPE_MISALIGNMENT => {
                                    println!("\tEXCEPTION_DATATYPE_MISALIGNMENT")
                                }
                                EXCEPTION_DEBUG_EVENT => println!("\tEXCEPTION_DEBUG_EVENT"),
                                EXCEPTION_FLT_DENORMAL_OPERAND => {
                                    println!("\tEXCEPTION_FLT_DENORMAL_OPERAND")
                                }
                                EXCEPTION_FLT_DIVIDE_BY_ZERO => {
                                    println!("\tEXCEPTION_FLT_DIVIDE_BY_ZERO")
                                }
                                EXCEPTION_FLT_INEXACT_RESULT => {
                                    println!("\tEXCEPTION_FLT_INEXACT_RESULT")
                                }
                                EXCEPTION_FLT_INVALID_OPERATION => {
                                    println!("\tEXCEPTION_FLT_INVALID_OPERATION")
                                }
                                EXCEPTION_FLT_OVERFLOW => println!("\tEXCEPTION_FLT_OVERFLOW"),
                                EXCEPTION_FLT_STACK_CHECK => {
                                    println!("\tEXCEPTION_FLT_STACK_CHECK")
                                }
                                EXCEPTION_FLT_UNDERFLOW => println!("\tEXCEPTION_FLT_UNDERFLOW"),
                                EXCEPTION_GUARD_PAGE => println!("\tEXCEPTION_GUARD_PAGE"),
                                EXCEPTION_ILLEGAL_INSTRUCTION => {
                                    println!("\tEXCEPTION_ILLEGAL_INSTRUCTION")
                                }
                                EXCEPTION_INT_DIVIDE_BY_ZERO => {
                                    println!("\tEXCEPTION_INT_DIVIDE_BY_ZERO")
                                }
                                EXCEPTION_INT_OVERFLOW => println!("\tEXCEPTION_INT_OVERFLOW"),
                                EXCEPTION_INVALID_DISPOSITION => {
                                    println!("\tEXCEPTION_INVALID_DISPOSITION")
                                }
                                EXCEPTION_INVALID_HANDLE => println!("\tEXCEPTION_INVALID_HANDLE"),
                                EXCEPTION_IN_PAGE_ERROR => println!("\tEXCEPTION_IN_PAGE_ERROR"),
                                EXCEPTION_NONCONTINUABLE_EXCEPTION => {
                                    println!("\tEXCEPTION_NONCONTINUABLE_EXCEPTION")
                                }
                                EXCEPTION_POSSIBLE_DEADLOCK => {
                                    println!("\tEXCEPTION_POSSIBLE_DEADLOCK")
                                }
                                EXCEPTION_PRIV_INSTRUCTION => {
                                    println!("\tEXCEPTION_PRIV_INSTRUCTION")
                                }
                                EXCEPTION_SINGLE_STEP | 1073741854 => {
                                    let _ = self.turnoff_single_step(self.current_tid as u32, None);
                                    if let Some(b) = self
                                        .reactivate_breakpoint
                                        .and_then(|index| self.breakpoints.get(index))
                                    {
                                        self.reactivate_breakpoint(b);

                                        match b {
                                            Breakpoint::Simple { go, ..} if *go => {
                                                self.break_on_next_single_step = false;
                                                continue;
                                            }
                                            _ => {}
                                        }
                                    }

                                    if self.break_on_next_single_step {
                                        self.break_on_next_single_step = false;
                                        break;
                                    } else {
                                        self.break_on_next_single_step = false;
                                    }
                                }
                                EXCEPTION_STACK_OVERFLOW => println!("\tEXCEPTION_STACK_OVERFLOw"),
                                winapi::um::winnt::DBG_CONTROL_C => println!("\tDBG_CONTROL_C"),
                                e @ _ => {
                                    panic!("Unkown exception code: {}", e);
                                }
                            }
                        }
                        EXIT_PROCESS_DEBUG_EVENT => {
                            debug!(target:"debugger", "EXIT_PROCESS_DEBUG_EVENT");
                            break;
                        }
                        EXIT_THREAD_DEBUG_EVENT => {
                            debug!(target:"debugger", "EXIT_THREAD_DEBUG_EVENT");
                        }
                        LOAD_DLL_DEBUG_EVENT => {
                            let info = unsafe { e.u.LoadDll() };

                            let imagename = unsafe {
                                let mut buffer = vec![0u8; 1024];
                                let _ = winapi::um::fileapi::GetFinalPathNameByHandleA(
                                    info.hFile,
                                    buffer.as_mut_ptr() as *mut i8,
                                    1024,
                                    0,
                                );
                                String::from_utf8(buffer).unwrap()
                            };
                            let filesize = unsafe {
                                let mut size = 0u32;
                                winapi::um::fileapi::GetFileSize(info.hFile, &mut size);
                                size
                            };
                            debug!(target:"debugger", "Loading @ {:X?}: {}", info.lpBaseOfDll, imagename.as_str());

                            let _ = self.modules.load_module(
                                info.lpBaseOfDll as usize,
                                filesize as usize,
                                imagename.as_str(),
                            );
                            self.try_resolve_breakpoints();
                        }
                        OUTPUT_DEBUG_STRING_EVENT => {
                            // println!("OUTPUT_DEBUG_STRING_EVENT");

                            let info = unsafe { e.u.DebugString() };
                            // lpDebugStringData: LPSTR
                            // fUnicode: WORD
                            // nDebugStringLength: WORD

                            let r = read_process_memory(
                                self.process,
                                info.lpDebugStringData as usize,
                                info.nDebugStringLength as usize,
                            )
                            .unwrap();
                            let r = std::ffi::CStr::from_bytes_with_nul(r.as_slice()).unwrap();
                            let r = r.to_str().unwrap();

                            debug!(target:"debugger", "Output: {}", r);
                        }
                        RIP_EVENT => {
                            debug!(target:"debugger", "RIP_EVENT");
                        }
                        UNLOAD_DLL_DEBUG_EVENT => {}
                        _ => {
                            debug!(target:"debugger", "Unknown debug event");
                        }
                    };
                }
                Err(_) => {
                    todo!();
                }
            };
        }
        trace!(target:"debugger", "go - end");
    }

    fn try_resolve_breakpoints(&mut self) {
        let mut still_unresolved = vec![];
        let mut f = self.unresolved_breakpoints.clone();

        for b in f.drain(..) {
            let resolved = match self.modules.get_function_addr("", &b.symbol) {
                Some(addr) => {
                    debug!(target:"debugger", "New breakpoint resolved: {:?} at 0x{:X}", b.symbol, addr);

                    if let Some(api) = self
                        .modules
                        .get_function_at(addr)
                        .and_then(|info| self.known_apis.get_by_name(&info.name))
                        .map(|x| x.clone())
                    {
                        self.add_breakpoint_knownapi(addr, api)
                    } else {
                        self.add_breakpoint_simple(addr, false);
                    }
                    true
                }
                None => false,
            };

            if !resolved {
                still_unresolved.push(b);
            }
        }

        self.unresolved_breakpoints = still_unresolved;
    }

    pub fn reactivate_breakpoint(&self, b: &Breakpoint) {
        let location = match b {
            Breakpoint::Simple { location, once, .. } => {
                if *once {
                    None
                } else {
                    Some(*location)
                }
            }
            Breakpoint::KnowApi { location, .. } => Some(*location),
            Breakpoint::Unresolved => None,
        };
        if let Some(location) = location {
            self.set_cc(location);
        }
    }

    pub fn restore_original(&self, b: &Breakpoint) {
        match b {
            Breakpoint::Simple {
                location,
                original_value,
                ..
            } => {
                let _ = write_process_memory(self.process, *location, original_value.as_slice())
                    .unwrap();
            }
            Breakpoint::KnowApi {
                location,
                original_value,
                ..
            } => {
                let _ = write_process_memory(self.process, *location, original_value.as_slice())
                    .unwrap();
            }
            Breakpoint::Unresolved => {}
        }
    }

    pub fn get_current_thread_context(&self) -> ThreadContext {
        let h = open_thread(
            OpenThreadAccess::GET_CONTEXT | OpenThreadAccess::SET_CONTEXT,
            false,
            self.current_tid as u32,
        )
        .unwrap();

        if is_wow64_process(h) {
            let ctx = super::wow64::get_thread_context(h).unwrap();
            ThreadContext {
                sp: ctx.Esp as u64,
                bp: ctx.Ebp as u64,
                ip: ctx.Eip as u64,
                ax: ctx.Eax as u64,
                bx: ctx.Ebx as u64,
                cx: ctx.Ecx as u64,
                dx: ctx.Edx as u64,
                si: ctx.Esi as u64,
                di: ctx.Edi as u64,
                dr6: ctx.Dr6 as u64,
            }
        } else {
            todo!();
        }
    }

    fn turnon_single_step(&self, tid: u32, addr: Option<u64>) -> Result<(), u32> {
        let h = open_thread(
            OpenThreadAccess::GET_CONTEXT | OpenThreadAccess::SET_CONTEXT,
            false,
            tid,
        )
        .unwrap();

        if is_wow64_process(h) {
            let mut ctx = super::wow64::get_thread_context(h).unwrap();
            if let Some(addr) = addr {
                ctx.Eip = addr as u32;
            }
            ctx.EFlags |= 0x100;
            super::wow64::set_thread_context(h, ctx).unwrap();
        } else {
            let mut ctx = get_thread_context(h)?;
            if let Some(addr) = addr {
                ctx.Rip = addr;
            }
            ctx.EFlags |= 0x100;
            set_thread_context(h, ctx)?;
        }

        Ok(())
    }

    fn turnoff_single_step(&self, tid: u32, addr: Option<u64>) -> Result<(), u32> {
        match open_thread(
            OpenThreadAccess::GET_CONTEXT | OpenThreadAccess::SET_CONTEXT,
            false,
            tid,
        ) {
            Ok(h) => {
                if is_wow64_process(h) {
                    let mut ctx = super::wow64::get_thread_context(h).unwrap();
                    if let Some(addr) = addr {
                        ctx.Eip = addr as u32;
                    }
                    ctx.EFlags &= !0x100;
                    super::wow64::set_thread_context(h, ctx).unwrap();
                } else {
                    let mut ctx = get_thread_context(h)?;
                    if let Some(addr) = addr {
                        ctx.Rip = addr;
                    }
                    ctx.EFlags &= !0x100;
                    set_thread_context(h, ctx)?;
                }
            }
            Err(_) => todo!(),
        }

        Ok(())
    }

    pub fn read_memory<T: Clone>(&self, addr: usize) -> Result<T, u32> {
        parse_at(addr, self.process)
    }

    pub fn read_array_memory<T: Clone>(&self, qty: usize, addr: usize) -> Vec<T> {
        parse_at_n(addr, self.process, qty).unwrap()
    }

    pub fn get_current_instruction(&self) -> Option<(usize, &iced_x86::Instruction)> {
        let ctx = self.get_current_thread_context();
        self.modules.get_instruction_at(ctx.ip as usize)
    }

    // pub fn op0_uses_mem(
    //     &self,
    //     ctx: &ThreadContext,
    //     i: &iced_x86::Instruction,
    //     mem_addr: usize,
    // ) -> bool {
    //     match i.op0_kind() {
    //         iced_x86::OpKind::Memory => {
    //             let base = ctx.get(i.memory_base());
    //             let displacement = i.memory_displacement32();
    //             let addr = base as usize + displacement as usize;
    //             addr == mem_addr
    //         }
    //         _ => false,
    //     }
    // }

    // pub fn op1_uses_mem(
    //     &self,
    //     ctx: &ThreadContext,
    //     i: &iced_x86::Instruction,
    //     mem_addr: usize,
    // ) -> bool {
    //     match i.op1_kind() {
    //         iced_x86::OpKind::Memory => {
    //             let base = ctx.get(i.memory_base());
    //             let displacement = i.memory_displacement32();
    //             let addr = base as usize + displacement as usize;
    //             addr == mem_addr
    //         }
    //         _ => false,
    //     }
    // }

    pub fn format_op0(&self, ctx: &ThreadContext, i: &iced_x86::Instruction, std: &mut String) {
        match i.op0_kind() {
            iced_x86::OpKind::Memory => {
                let base = ctx.get(i.memory_base());
                let displacement = i.memory_displacement32();
                let addr = base as usize + displacement as usize;
                let v: Result<u32, u32> = parse_at(addr, self.process);
                std.push_str(format!(" - mem[{}]={:?}", addr, v).as_str());
            }
            iced_x86::OpKind::Register => {
                let r = i.op0_register();
                let v = ctx.get(r);
                std.push_str(format!(" - {:?}={}", r, v).as_str());
            }
            _ => {}
        }
    }

    pub fn format_op1(&self, ctx: &ThreadContext, i: &iced_x86::Instruction, std: &mut String) {
        match i.op1_kind() {
            iced_x86::OpKind::Memory => {
                let base = ctx.get(i.memory_base());
                let displacement = i.memory_displacement32();
                let addr = base as usize + displacement as usize;
                let v: Result<u32, u32> = parse_at(addr, self.process);
                std.push_str(format!("- mem[{}]={:?}", addr, v).as_str());
            }
            iced_x86::OpKind::Register => {
                let r = i.op1_register();
                let v = ctx.get(r);
                std.push_str(format!("- {:?}={}", r, v).as_str());
            }
            _ => {}
        }
    }

    pub fn format_op0_as_float(
        &self,
        ctx: &ThreadContext,
        i: &iced_x86::Instruction,
        std: &mut String,
    ) {
        match i.op0_kind() {
            iced_x86::OpKind::Memory => {
                let base = ctx.get(i.memory_base());
                let displacement = i.memory_displacement32();
                let addr = base as usize + displacement as usize;
                let v: Result<u32, u32> = parse_at(addr, self.process);
                if let Ok(v) = v {
                    let v: f32 = unsafe { std::mem::transmute(v) };
                    std.push_str(format!(" - mem[{}]={:?}", addr, v).as_str());
                }
            }
            _ => {}
        }
    }

    // pub fn uses_mem(&self, addr: usize) -> bool {
    //     match self.get_current_instruction() {
    //         Some((_, i)) => {
    //             let ctx = self.get_current_thread_context();
    //             self.op0_uses_mem(&ctx, i, addr) || self.op1_uses_mem(&ctx, i, addr)
    //         }
    //         None => false,
    //     }
    // }

    pub fn format_instruction(&self, i: &iced_x86::Instruction) -> String {
        let ctx = self.get_current_thread_context();

        use iced_x86::Formatter;
        let mut output = String::new();
        let mut formatter = iced_x86::NasmFormatter::new();
        formatter.format(&i, &mut output);

        use iced_x86::Mnemonic::*;
        match i.mnemonic() {
            Mov | Movzx | Add | Sub | Xor | Mul | Imul | And | Or | Shl | Shr | Test | Cmp
            | Lea => {
                self.format_op0(&ctx, i, &mut output);
                self.format_op1(&ctx, i, &mut output);
            }
            Push | Pop => {
                self.format_op0(&ctx, i, &mut output);
            }
            Fld => {
                self.format_op0_as_float(&ctx, i, &mut output);
            }
            Fst | Fstp => {
                self.format_op0_as_float(&ctx, i, &mut output);
            }
            _ => {}
        }

        output
    }

    pub fn get_function_at(&self, addr: usize) -> Option<KnownCall> {
        let f = self.modules.get_function_at(addr)?;
        match self.known_apis.get_by_name(&f.name).map(Clone::clone) {
            Some(f) => Some(f.parse_know_call(self.process, self.current_tid as u32)),
            None => Some(KnownCall {
                name: f.name.clone(),
                args: Default::default(),
            }),
        }
    }

    pub fn trace_function_at(&mut self, addr: usize) -> Option<()> {
        trace!("trace_function_at: {:X}", addr);

        
        let (mut addr, instructions) = self.modules.get_instructions_at(addr)?.clone();
        debug!("{:X}, {}",  addr, instructions.len());
        for i in instructions {
            self.add_breakpoint_trace(addr, false);
            addr += i.len();
        }

        Some(())
    }
}

mod debugger;
mod helpers;
pub mod known_api;
mod modules;
mod w32;
mod wow64;

use std::io::{Write, stdout};
use flume::*;
use known_api::*;
use self::{debugger::ThreadContext};

#[derive(Debug)]
pub enum Commands {
    Init(String, Sender<()>),
    Go(Sender<()>),
    GoUntilUsesMem(usize, Sender<()>),
    Step(Sender<()>),
    AddUnresolvedBreakpoint(String, bool, Sender<usize>), // symbol, once
    AddBreakpoint(usize, bool, Sender<usize>),            // location, once
    AddMemoryBreakpoint(usize, Sender<usize>),            // location
    Print(Vec<serde_json::Value>, Sender<()>),
    CurrentStackFrame(Sender<Option<KnownCall>>),
    GetThreadContext(Sender<Option<ThreadContext>>),
    ReadMemory(String, usize, Sender<serde_json::Value>), // type, addr
    ReadArrayMemory(String, usize, usize, Sender<serde_json::Value>), // type, n, addr
    GetCurrentInstructionString(Sender<String>),
    WriteFile(String, Vec<u8>, Sender<()>),
    GetFunctionAt(u64, Sender<KnownCall>),
    TraceFunctionAt(u64, Sender<()>),
}

pub fn spawn(cmds: Receiver<Commands>) {
    let (s, r) = unbounded();

    let _ = std::thread::spawn(move || -> ! {
        let f = stdout();
        let mut f = std::io::BufWriter::new(f);
        let mut dbg = debugger::Debugger::new();
        loop {
            let cmd = r.recv();
            match cmd {
                Ok(Commands::Init(path, callback)) => {
                    dbg.start(path.as_str());
                    dbg.go();
                    let _ = callback.send(());
                }
                Ok(Commands::Go(callback)) => {
                    dbg.go();
                    let _ = callback.send(());
                }
                Ok(Commands::GoUntilUsesMem(addr, _)) => {
                    let _ = dbg.add_breakpoint_memory(addr);
                    loop {
                        dbg.go();

                        let ctx = dbg.get_current_thread_context();
                        if ctx.dr6 != 0 {
                            break;
                        }

                        // if let Some((_, i)) = dbg.get_current_instruction() {
                        //     let call = dbg.get_function_at(addr as usize).unwrap_or_default();
                        //     write!(f, "{:?}\n", call);

                        //     let s = dbg.format_instruction(&i);
                        //     write!(f, "{}\n", format!("0x{:X} {}", addr, s));
                        //     if dbg.uses_mem(addr) {
                        //         break;
                        //     }
                        // }
                    }
                    // loop {
                    //     dbg.step();

                    //     let ctx = dbg.get_current_thread_context();
                    //     if ctx.ip > 0x70000000 {
                    //         let addr: u32 = dbg.read_memory(ctx.sp as usize);
                    //         dbg.add_breakpoint_simple(addr as usize, true);
                    //     }

                    //     if let Some((addr, i)) =
                    //         dbg.get_current_instruction().map(|(x, i)| (x, i.clone()))
                    //     {

                    //     }
                    // }
                    // let _ = callback.send(());
                }
                Ok(Commands::Step(callback)) => {
                    dbg.step();
                    let _ = callback.send(());
                }
                Ok(Commands::AddUnresolvedBreakpoint(symbol, once, callback)) => {
                    let i = if let Ok(addr) = usize::from_str_radix(symbol.as_str(), 16) {
                        dbg.add_breakpoint_simple(addr, once)
                    } else {
                        dbg.add_breakpoint_symbol("", symbol.as_str())
                        //TODO once
                    };

                    let _ = callback.send(i);
                }
                Ok(Commands::AddBreakpoint(location, once, callback)) => {
                    let i = dbg.add_breakpoint_simple(location, once);
                    let _ = callback.send(i);
                }
                Ok(Commands::AddMemoryBreakpoint(location, callback)) => {
                    let i = dbg.add_breakpoint_memory(location);
                    let _ = callback.send(i);
                }
                Ok(Commands::Print(arguments, callback)) => {
                    // use json_color::Colorizer;
                    // let colorizer = Colorizer::arbitrary();
                    for arg in arguments {
                        match arg {
                            serde_json::Value::String(s) => {
                                let _ = write!(f, "{} ", s);
                            }
                            serde_json::Value::Number(n) => {
                                let s = format!("{}", n);
                                let _ = write!(f, "{} ", s);
                            }
                            x => {
                                let s = x.to_string();
                                let _ = write!(f, "{} ", s);
                            }
                        };
                    }
                    let _ = write!(f, "\n");
                    let _ = f.flush();
                    let _ = callback.send(());
                }
                Ok(Commands::CurrentStackFrame(callback)) => {
                    let call = dbg.get_current_known_call().map(|x| x.clone());
                    let _ = callback.send(call);
                }
                Ok(Commands::GetThreadContext(callback)) => {
                    let ctx = dbg.get_current_thread_context();
                    let _ = callback.send(Some(ctx));
                }
                Ok(Commands::GetCurrentInstructionString(callback)) => {
                    let s = if let Some((addr, i)) = dbg.get_current_instruction() {
                        let s = dbg.format_instruction(i);
                        format!("0x{:X} {}", addr, s)
                    } else {
                        format!("<ERROR>")
                    };
                    let _ = callback.send(s);
                }
                Ok(Commands::ReadMemory(t, addr, callback)) => {
                    let v = match t.as_str() {
                        "u8" | "U8" => {
                            dbg.read_memory::<u8>(addr)
                                .map(|v| serde_json::Value::Number(v.into()))
                                .unwrap_or(serde_json::Value::Null)
                        }
                        "u16" | "U16" => {
                            dbg.read_memory::<u16>(addr)
                            .map(|v| serde_json::Value::Number(v.into()))
                            .unwrap_or(serde_json::Value::Null)
                        }
                        "u32" | "U32" => {
                            dbg.read_memory::<u32>(addr)
                            .map(|v| serde_json::Value::Number(v.into()))
                            .unwrap_or(serde_json::Value::Null)
                        }
                        "f32" | "F32" => {
                            dbg.read_memory::<f32>(addr)
                            .map(|v| serde_json::json!{v})
                            .unwrap_or(serde_json::Value::Null)
                        }
                        _ => todo!(),
                    };
                    let _ = callback.send(v);
                }
                Ok(Commands::ReadArrayMemory(t, qty, addr, callback)) => {
                    let v = match t.as_str() {
                        "u8" | "U8" => {
                            let v: Vec<serde_json::Value> = dbg
                                .read_array_memory::<u8>(qty, addr)
                                .iter()
                                .map(|&x| serde_json::Value::Number(x.into()))
                                .collect();
                            serde_json::Value::Array(v)
                        }
                        "u16" | "U16" => {
                            let v: Vec<serde_json::Value> = dbg
                                .read_array_memory::<u16>(qty, addr)
                                .iter()
                                .map(|&x| serde_json::Value::Number(x.into()))
                                .collect();
                            serde_json::Value::Array(v)
                        }
                        "u32" | "U32" => {
                            let v: Vec<serde_json::Value> = dbg
                                .read_array_memory::<u32>(qty, addr)
                                .iter()
                                .map(|&x| serde_json::Value::Number(x.into()))
                                .collect();
                            serde_json::Value::Array(v)
                        }
                        "f32" | "F32" => {
                            let v: Vec<serde_json::Value> = dbg
                                .read_array_memory::<f32>(qty, addr)
                                .iter()
                                .map(|&x| serde_json::json!(x as f64))
                                .collect();
                            serde_json::Value::Array(v)
                        }
                        _ => todo!(),
                    };
                    let _ = callback.send(v);
                }
                Ok(Commands::WriteFile(path, bytes, callback)) => {
                    let _ = std::fs::write(path, bytes.as_slice()).unwrap();
                    let _ = callback.send(());
                }
                Ok(Commands::GetFunctionAt(addr, callback)) => {
                    let f = dbg.get_function_at(addr as usize).unwrap_or_default();
                    let _ = callback.send(f);
                }
                Ok(Commands::TraceFunctionAt(addr, callback)) => {
                    dbg.trace_function_at(addr as usize);
                    let _ = callback.send(());
                }
                Err(_) => todo!(),
            }
        }
    });

    let _ = std::thread::spawn(move || loop {
        let cmd = cmds.recv();
        match cmd {
            Ok(c) => {
                let _ = s.send(c);
            }
            x @ Err(_) => todo!("{:?}", x),
        }
    });
}

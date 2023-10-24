#![feature(try_blocks)]
#![feature(concat_idents)]

mod debugger;
mod script;
use debugger::*;
use flume::*;
use log::debug;
use script::Script;
use structopt::*;

async fn jsevent_to_dbgcmd(
    script: Sender<script::Commands>,
    script_events: Receiver<script::Events>,
    dbg: Sender<debugger::Commands>,
) {
    loop {
        let msg = script_events.recv_async().await;
        debug!("jsevent_to_dbgcmd {:?}", msg);
        match msg {
            Ok(script::Events::NativeCode(resolver, f, arguments)) => {
                let _ = match f.as_str() {
                    "init" => {
                        let (s, r) = bounded(1);
                        let arg0 = arguments[0].as_str().unwrap();
                        let _ = dbg.send(Commands::Init(arg0.to_string(), s));
                        let _ = r.recv_async().await;
                        let _ = script
                            .send(script::Commands::Resolve(resolver, serde_json::Value::Null));
                    }
                    "go" => {
                        let (s, r) = bounded(1);
                        let _ = dbg.send(Commands::Go(s));
                        let _ = r.recv_async().await;
                        let _ = script
                            .send(script::Commands::Resolve(resolver, serde_json::Value::Null));
                    }
                    "goUntilUsesMem" => {
                        let addr = arguments[0].as_u64().unwrap() as usize;

                        let (s, r) = bounded(1);
                        let _ = dbg.send(Commands::GoUntilUsesMem(addr, s));
                        let _ = r.recv_async().await;
                        let _ = script
                            .send(script::Commands::Resolve(resolver, serde_json::Value::Null));
                    }
                    "step" => {
                        let (s, r) = bounded(1);
                        let _ = dbg.send(Commands::Step(s));
                        let _ = r.recv_async().await;
                        let _ = script
                            .send(script::Commands::Resolve(resolver, serde_json::Value::Null));
                    }
                    "addBreakpoint" => {
                        let (s, r) = bounded(1);

                        let once = if arguments.len() >= 2 {
                            arguments[1]["once"].as_bool().unwrap_or(false)
                        } else {
                            false
                        };

                        if arguments[0].is_string() {
                            let arg0 = arguments[0].as_str().unwrap();
                            let _ = dbg.send(Commands::AddUnresolvedBreakpoint(
                                arg0.to_string(),
                                once,
                                s,
                            ));
                        } else if arguments[0].is_number() {
                            let arg0 = arguments[0].as_u64().unwrap();
                            let _ = dbg.send(Commands::AddBreakpoint(arg0 as usize, once, s));
                        } else {
                            todo!()
                        }
                        let _ = r.recv_async().await;
                        let _ = script
                            .send(script::Commands::Resolve(resolver, serde_json::Value::Null));
                    }
                    "addMemoryBreakpoint" => {
                        let addr = arguments[0].as_u64().unwrap() as usize;
                        let (s, r) = bounded(1);
                        let _ = dbg.send(Commands::AddMemoryBreakpoint(addr, s));
                        let _ = r.recv_async().await;
                        let _ = script
                            .send(script::Commands::Resolve(resolver, serde_json::Value::Null));
                    }
                    "print" => {
                        let (s, r) = bounded(1);
                        let _ = dbg.send(Commands::Print(arguments, s));
                        let _ = r.recv_async().await;
                        let _ = script
                            .send(script::Commands::Resolve(resolver, serde_json::Value::Null));
                    }
                    "currentStackFrame" => {
                        let (s, r) = bounded(1);
                        let _ = dbg.send(Commands::CurrentStackFrame(s));
                        match r.recv_async().await {
                            Ok(Some(call)) => {
                                let json = serde_json::to_value(call).unwrap();
                                let _ = script.send(script::Commands::Resolve(resolver, json));
                            },
                            _ => {
                                let json = serde_json::Value::Null;
                                let _ = script.send(script::Commands::Resolve(resolver, json));
                            }
                        };                      
                    }
                    "getThreadContext" => {
                        let (s, r) = bounded(1);
                        let _ = dbg.send(Commands::GetThreadContext(s));
                        let r = r.recv_async().await.unwrap().unwrap();
                        let r = serde_json::to_value(r).unwrap();
                        let _ = script.send(script::Commands::Resolve(resolver, r));
                    }
                    "read" => {
                        let (s, r) = bounded(1);

                        let t = arguments[0].as_str().unwrap().to_string();
                        let addr = arguments[1].as_u64().unwrap() as usize;

                        let _ = dbg.send(Commands::ReadMemory(t, addr, s));
                        let r = r.recv_async().await.unwrap();
                        let _ = script.send(script::Commands::Resolve(resolver, r));
                    }
                    "readArray" => {
                        let (s, r) = bounded(1);

                        let t = arguments[0].as_str().unwrap().to_string();
                        let qty = arguments[1].as_u64().unwrap() as usize;
                        let addr = arguments[2].as_u64().unwrap() as usize;

                        let _ = dbg.send(Commands::ReadArrayMemory(t, qty, addr, s));
                        let r = r.recv_async().await.unwrap();
                        let _ = script.send(script::Commands::Resolve(resolver, r));
                    }
                    "getCurrentInstructionString" => {
                        let (s, r) = bounded(1);

                        let _ = dbg.send(Commands::GetCurrentInstructionString(s));
                        let s = r.recv_async().await.unwrap();
                        let _ = script.send(script::Commands::Resolve(resolver, s.into()));
                    }
                    "writeFile" => {
                        let (s, r) = bounded(1);

                        let path = arguments[0].as_str().unwrap().to_string();
                        let arr: Vec<u8> = arguments[1]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|x| x.as_u64().unwrap() as u8)
                            .collect();

                        let _ = dbg.send(Commands::WriteFile(path, arr, s));
                        let _ = r.recv_async().await.unwrap();
                        let _ = script
                            .send(script::Commands::Resolve(resolver, serde_json::Value::Null));
                    }
                    "getFunctionAt" => {
                        let (s, r) = bounded(1);

                        let addr = arguments[0].as_u64().unwrap();
                        let _ = dbg.send(Commands::GetFunctionAt(addr, s));
                        let f = r.recv_async().await.unwrap();
                        let r = serde_json::to_value(f).unwrap();
                        let _ = script.send(script::Commands::Resolve(resolver, r));
                    }
                    "traceFunction" => {
                        let (s, r) = bounded(1);

                        let addr = arguments[0].as_u64().unwrap();
                        let _ = dbg.send(Commands::TraceFunctionAt(addr, s));
                        let f = r.recv_async().await.unwrap();
                        let r = serde_json::to_value(f).unwrap();
                        let _ = script.send(script::Commands::Resolve(resolver, r));
                    }
                    _ => todo!(),
                };
            }
            Err(_) => todo!(),
        }
    }
}

#[derive(StructOpt, Debug)]
pub struct Args {
    #[structopt(short, long)]
    path: Option<String>,

    #[structopt(short, long)]
    verbose: bool,
}

async fn run_repl(_: Args, mut script: Script) {
    let mut rl = rustyline::Editor::<()>::new();
    let _ = rl.load_history("~/.milkdbghistory.txt");
    loop {
        let readline = rl.readline("> ");
        match readline {
            Ok(src) => {
                rl.add_history_entry(src.as_str());
                let (s, r) = bounded::<()>(1);
                script.send_async(script::Commands::Run(src, s)).await;
                let _ = r.recv_async().await;
            }
            Err(_) => break,
        }
    }
    let _ = rl.append_history("~/.milkdbghistory.txt");
}

async fn run_script(args: Args, mut script: Script) {
    let src = std::fs::read(args.path.unwrap()).unwrap();
    let src = String::from_utf8(src).unwrap();

    if args.verbose {
        println!("Script");
        println!("----------------------------------");
        println!("{}", src);
        println!("----------------------------------");
    }

    let (s, r) = bounded::<()>(1);
    script.send_async(script::Commands::Run(src, s)).await;
    let _ = r.recv_async().await;
}

#[async_std::main]
async fn main() {
    let args = Args::from_args();
    if args.verbose {
        println!("{:#?}", args);
    }

    pretty_env_logger::init();

    let (jsevents_sender, jsevents_recv) = flume::unbounded();
    let (dbgcmd_sender, dbgcmd_recv) = flume::unbounded();

    debugger::spawn(dbgcmd_recv);
    let script = script::start(jsevents_sender);
    async_std::task::spawn(jsevent_to_dbgcmd(
        script.sender.clone(),
        jsevents_recv,
        dbgcmd_sender,
    ));

    if args.path.is_none() {
        run_repl(args, script).await;
    } else {
        run_script(args, script).await;
    }
}

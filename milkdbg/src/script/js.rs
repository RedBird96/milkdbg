use flume::*;
use log::debug;
use rusty_v8 as v8;
use v8::{FunctionCallback, Local, MapFnTo};

use super::Events;
static mut ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

macro_rules! gen_method {
    ($scope:ident, $global:ident, $s:ident, $name:ident) => {
        concat_idents::concat_idents! {fn_name = $name, _callback {
            #[allow(non_snake_case)]
            fn fn_name(
                scope: &mut v8::HandleScope,
                args: v8::FunctionCallbackArguments,
                mut retval: v8::ReturnValue,
            ) {
                let s = unsafe { v8::Local::<v8::External>::cast(args.data().unwrap()) };
                let s = unsafe { &mut *(s.value() as *mut flume::Sender<Events>) };

                let mut vargs = vec![];
                for _ in 0..args.length() {
                    let arg = args.get(vargs.len() as i32);
                    let v = v8::json::stringify(scope, arg).unwrap();
                    let v = v.to_rust_string_lossy(scope);
                    let v: serde_json::Value = serde_json::from_str(v.as_str()).unwrap();
                    vargs.push(v);
                }

                let resolver = v8::PromiseResolver::new(scope).unwrap();
                let promise = resolver.get_promise(scope);

                let context = scope.get_current_context();
                let global = context.global(scope);

                let id = unsafe { ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst) };
                global.set_index(scope, id, resolver.into()).unwrap();

                let msg = Events::NativeCode(id, stringify!($name).to_string(), vargs);
                debug!(target:"script", "{:?}", msg);
                let _ = s.send(msg);

                retval.set(promise.into());
            }

            {
                let f = new_method(&mut $scope, $s.clone(), fn_name);
                let name = v8::String::new(&mut $scope, stringify!($name)).unwrap();
                $global.set(&mut $scope, name.into(), f.into()).unwrap();
            }
        }}
    };
}

fn call_callback_callback(
    _: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    _: v8::ReturnValue,
) {
    let s = unsafe { v8::Local::<v8::External>::cast(args.data().unwrap()) };
    let s = unsafe { &*(s.value() as *mut flume::Sender<()>) }; //todo this is leaking
    let _ = s.send(());
}

fn new_method<'a, T, F: MapFnTo<FunctionCallback>>(
    scope: &mut v8::HandleScope<'a>,
    s: flume::Sender<T>,
    callback: F,
) -> Local<'a, v8::Function> {
    let s = Box::leak(Box::new(s));

    let external = v8::External::new(scope, s as *mut Sender<T> as *mut std::ffi::c_void);
    let f = v8::Function::builder(callback)
        .data(external.into())
        .build(scope)
        .unwrap();
    f
}

pub struct JavascriptEngine;

impl JavascriptEngine {
    pub fn spawn(sender: Sender<Events>) -> Sender<super::Commands> {
        let (s, r) = unbounded::<super::Commands>();
        std::thread::spawn(move || {
            let platform = v8::new_default_platform(0, false).make_shared();
            v8::V8::initialize_platform(platform);
            v8::V8::initialize();
            // let v = v8::V8::set_flags_from_command_line(vec![
            //     "".to_string(),
            //     "--harmony-top-level-await".to_string(),
            // ]);
            // println!("{:?}", v);

            let mut isolate = v8::Isolate::new(v8::CreateParams::default());
            let mut handle_scope = v8::HandleScope::new(&mut isolate);

            let s = Box::leak(Box::new(sender));
            let context = v8::Context::new(&mut handle_scope);
            let mut scope = v8::ContextScope::new(&mut handle_scope, context);
            let global = context.global(&mut scope);

            gen_method! {scope, global, s, init}
            gen_method! {scope, global, s, go}
            gen_method! {scope, global, s, goUntilUsesMem}
            gen_method! {scope, global, s, step}
            gen_method! {scope, global, s, addBreakpoint}
            gen_method! {scope, global, s, addMemoryBreakpoint}
            gen_method! {scope, global, s, print}
            gen_method! {scope, global, s, currentStackFrame}
            gen_method! {scope, global, s, getThreadContext}
            gen_method! {scope, global, s, read}
            gen_method! {scope, global, s, readArray}
            gen_method! {scope, global, s, getCurrentInstructionString}
            gen_method! {scope, global, s, writeFile}
            gen_method! {scope, global, s, getFunctionAt}
            gen_method! {scope, global, s, traceFunction}

            loop {
                let code = r.recv();

                match code {
                    Ok(crate::script::Commands::Resolve(index, value)) => {
                        let result = serde_json::to_string(&value).unwrap();

                        // println!("{}", result.to_colored_json_auto().unwrap()); //TODO only on REPL

                        let result = v8::String::new(&mut scope, result.as_str()).unwrap();
                        let result = v8::json::parse(&mut scope, result).unwrap();

                        let resolver = global.get_index(&mut scope, index).unwrap();
                        let resolver = unsafe { v8::Local::<v8::PromiseResolver>::cast(resolver) };
                        resolver.resolve(&mut scope, result);

                        global.delete_index(&mut scope, index);
                    }
                    Ok(crate::script::Commands::Run(code, callback)) => {
                        let code = v8::String::new(&mut scope, &code).unwrap();
                        if let Some(script) = v8::Script::compile(&mut scope, code, None) {
                            if let Some(result) = script.run(&mut scope) {
                                if result.is_promise() {
                                    let p = unsafe { v8::Local::<v8::Promise>::cast(result) };
                                    let f = new_method(
                                        &mut scope,
                                        callback.clone(),
                                        call_callback_callback,
                                    );
                                    p.then(&mut scope, f);
                                } else {
                                    let _ = callback.send(());
                                }
                            }
                        } else {
                            println!("Error compiling");
                        }
                    }
                    _ => todo!(),
                }
            }
        });

        s
    }
}

use std::collections::HashMap;

use super::{helpers::*, w32::*};
use include_dir::*;
use serde::*;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[allow(dead_code)]
pub struct KnownCall {
    pub name: String,
    pub args: serde_json::Map<String, serde_json::Value>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum KnownApiArgLocation {
    Memory(iced_x86::Register, isize),
}

fn get_register_value(ctx: winapi::um::winnt::WOW64_CONTEXT, register: &iced_x86::Register) -> u32 {
    match register {
        iced_x86::Register::ESP => ctx.Esp,
        _ => todo!(),
    }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum KnownApiArgType {
    U32,
    UTF8String,
    UTF16String,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct KnownApiArg {
    pub location: KnownApiArgLocation,
    pub t: KnownApiArgType,
    pub name: String,
}

impl KnownApiArg {
    pub fn get_value(
        &self,
        process: winapi::um::winnt::HANDLE,
        ctx: winapi::um::winnt::WOW64_CONTEXT,
    ) -> serde_json::Value {
        let addr = match &self.location {
            KnownApiArgLocation::Memory(register, offset) => {
                let addr = get_register_value(ctx, register);
                ((addr as isize) - offset) as usize
            }
        };

        match self.t {
            KnownApiArgType::U32 => {
                let n: u32 = parse_at(addr, process).unwrap();
                serde_json::Value::Number(n.into())
            }
            KnownApiArgType::UTF8String => {
                let addr: u32 = parse_at(addr, process).unwrap();
                serde_json::Value::String(
                    read_utf8_string_char_by_char_unchecked(process, addr as usize).unwrap(),
                )
            }
            KnownApiArgType::UTF16String => {
                let addr: u32 = parse_at(addr, process).unwrap();
                serde_json::Value::String(
                    read_utf16_string_char_by_char_unchecked(process, addr as usize).unwrap(),
                )
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct KnownApi {
    pub name: String,
    pub args: Vec<KnownApiArg>,
}

impl KnownApi {
    pub fn parse_know_call(&self, process: winapi::um::winnt::HANDLE, tid: u32) -> KnownCall {
        //TODO test is wow64
        // see turn_single_step
        let handle = open_thread(
            OpenThreadAccess::GET_CONTEXT | OpenThreadAccess::SET_CONTEXT,
            false,
            tid,
        )
        .unwrap();

        let ctx = super::wow64::get_thread_context(handle).unwrap();
        KnownCall {
            name: self.name.clone(),
            args: self
                .args
                .iter()
                .map(|x| (x.name.clone(), x.get_value(process, ctx)))
                .collect(),
        }
    }
}

static APIS: Dir = include_dir!("./apis");

pub struct KnownApiDatabase {
    by_name: HashMap<String, KnownApi>,
}

impl KnownApiDatabase {
    pub fn new() -> Self {
        let mut by_name = HashMap::new();

        let glob = "*.json";
        for entry in APIS.find(glob).unwrap() {
            match entry {
                DirEntry::File(file) => {
                    let j = file.contents_utf8().unwrap();
                    let j = json::parse(&j).unwrap();
                    for f in j["Functions"].members() {
                        let name = f["Name"].as_str().unwrap().to_string();

                        let mut args = vec![];
                        let mut offset = -4isize;
                        for p in f["Params"].members() {
                            let name = p["Name"].as_str().unwrap().to_string();
                            let t = p["Type"]["Name"].as_str().unwrap_or("NOTYPE").to_string();
                            let t = match t.as_str() {
                                "PSTR" => KnownApiArgType::UTF8String,
                                "PWSTR" => KnownApiArgType::UTF16String,
                                _ => KnownApiArgType::U32,
                            };
                            args.push(KnownApiArg {
                                name,
                                t,
                                location: KnownApiArgLocation::Memory(
                                    iced_x86::Register::ESP,
                                    offset,
                                ),
                            });
                            offset -= 4;
                        }

                        by_name.insert(name.clone(), KnownApi { name, args });
                    }
                }
                _ => {}
            }
        }

        Self { by_name }
    }

    pub fn get_by_name(&self, name: &str) -> Option<&KnownApi> {
        self.by_name.get(name)
    }
}

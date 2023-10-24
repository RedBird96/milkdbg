use super::w32::*;
use std::convert::TryInto;

pub fn string_from_array_with_zero(s: &[i8]) -> String {
    let s: &[u8] = unsafe { std::mem::transmute(s) };
    let (i, _) = s.iter().enumerate().find(|x| *x.1 == 0).unwrap();
    let s = std::ffi::CStr::from_bytes_with_nul(&s[0..i + 1]).unwrap();
    s.to_str().unwrap().to_string()
}

pub fn parse_at_n<T: Clone>(
    mut addr: usize,
    process: winapi::um::winnt::HANDLE,
    n: usize,
) -> Result<Vec<T>, u32> {
    let mut items = vec![];
    for _ in 0..n {
        let buffer = read_process_memory(process, addr, std::mem::size_of::<T>())?;
        let v: &T = unsafe { std::mem::transmute(buffer.as_ptr()) };
        let v = (*v).clone();
        items.push(v);
        addr += std::mem::size_of::<T>();
    }
    Ok(items)
}

pub fn parse_at<T: Clone>(addr: usize, process: winapi::um::winnt::HANDLE) -> Result<T, u32> {
    let buffer = read_process_memory(process, addr, std::mem::size_of::<T>())?;
    let v: &T = unsafe { std::mem::transmute(buffer.as_ptr()) };
    Ok((*v).clone())
}

#[allow(dead_code)]
pub fn parse<T: Clone>(addr: &mut usize, process: winapi::um::winnt::HANDLE) -> T {
    let buffer = read_process_memory(process, *addr, std::mem::size_of::<T>()).unwrap();
    let v: &T = unsafe { std::mem::transmute(buffer.as_ptr()) };
    *addr += std::mem::size_of::<T>();
    (*v).clone()
}

#[allow(dead_code)]
pub fn read_unicode_string_char_by_char(
    process: winapi::um::winnt::HANDLE,
    mut addr: usize,
) -> Result<String, std::string::FromUtf16Error> {
    if addr == 0 {
        return Ok("".to_string());
    }
    let mut name = vec![];
    loop {
        let c = read_process_memory(process, addr, 2).unwrap();
        if c[0] == 0 && c[1] == 0 {
            break;
        } else {
            let v = u16::from_le_bytes(c[0..2].try_into().unwrap());
            name.push(v);
            addr += 2;
        }
    }
    println!("\tname len: {}", name.len());
    String::from_utf16(name.as_slice())
}

pub fn read_string_char_by_char(
    process: winapi::um::winnt::HANDLE,
    mut addr: usize,
) -> Result<String, std::string::FromUtf8Error> {
    if addr == 0 {
        return Ok("".to_string());
    }
    let mut name = vec![];
    loop {
        let c = read_process_memory(process, addr, 1).unwrap();
        if c[0] == 0 {
            break;
        } else {
            name.push(c[0]);
            addr += 1;
        }
    }
    String::from_utf8(name)
}

pub fn read_utf8_string_char_by_char_unchecked(
    process: winapi::um::winnt::HANDLE,
    mut addr: usize,
) -> Result<String, u32> {
    if addr == 0 {
        return Ok("".to_string());
    }
    let mut name = vec![];
    loop {
        let c = read_process_memory(process, addr, 1)?;
        if c[0] < 32 || c[0] >= 127 {
            break;
        } else {
            name.push(c[0]);
            addr += 1;
        }
    }
    unsafe { Ok(String::from_utf8_unchecked(name)) }
}


pub fn read_utf16_string_char_by_char_unchecked(
    process: winapi::um::winnt::HANDLE,
    mut addr: usize,
) -> Result<String, u32> {
    if addr == 0 {
        return Ok("".to_string());
    }
    let mut s = vec![];
    loop {
        if s.len() >= 1024 {
            break;
        }
        let c = read_process_memory(process, addr, 2)?;
        if c[0] == 0 && c[1] == 0 {
            break;
        } else {
            let v = u16::from_le_bytes([c[0], c[1]]); 
            s.push(v);
            addr += 2;
        }
    }
    Ok(String::from_utf16(&s).unwrap())
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Value {
    U32(u32),
    String(String),
}

#[allow(dead_code)]
pub fn read_value_from_stack(
    process: winapi::um::winnt::HANDLE,
    ctx: &winapi::um::winnt::WOW64_CONTEXT,
    i: usize,
) -> Result<Value, u32> {
    let addr = ctx.Esp as usize;
    let data = read_process_memory(process, addr + ((i + 1) * 4), 4).unwrap();

    let v = u32::from_le_bytes(data[0..4].try_into().unwrap());
    Ok(Value::U32(v))
}

#[allow(dead_code)]
pub fn read_value_from_stack_map<F, T>(
    process: winapi::um::winnt::HANDLE,
    ctx: &winapi::um::winnt::WOW64_CONTEXT,
    i: usize,
    f: F,
) -> Result<T, u32>
where
    F: Fn(Vec<u8>) -> T,
{
    let addr = ctx.Esp as usize;
    let data = read_process_memory(process, addr + ((i + 1) * 4), 4).unwrap();
    Ok(f(data))
}

#[allow(dead_code)]
pub fn read_value_from_stack_as_ptr_to_string(
    process: winapi::um::winnt::HANDLE,
    ctx: &winapi::um::winnt::WOW64_CONTEXT,
    i: usize,
) -> Result<Value, u32> {
    let addr = ctx.Esp as usize;
    let ptr = parse_at::<u32>(addr + ((i + 1) * 4), process)?;
    let s = read_utf8_string_char_by_char_unchecked(process, ptr as usize)?;
    Ok(Value::String(s))
}

#[allow(dead_code)]
pub fn try_read_string_char_by_char(
    process: winapi::um::winnt::HANDLE,
    mut addr: usize,
) -> Result<String, ()> {
    if addr == 0 {
        return Ok("".to_string());
    }
    let mut name = vec![];
    for _ in 0..16 {
        match read_process_memory(process, addr, 1) {
            Ok(c) => {
                if c[0] < 32 || c[0] >= 127 {
                    return Err(());
                } else {
                    name.push(c[0]);
                    addr += 1;
                }
            }
            Err(_) => break,
        }
    }
    String::from_utf8(name).map_err(|_| ())
}

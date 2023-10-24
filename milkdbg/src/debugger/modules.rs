use super::helpers::*;
use super::w32::*;
use iced_x86::Instruction;
use log::debug;
use rust_lapper::*;

type Iv = Interval<usize, usize>;

#[derive(Debug)]
#[allow(dead_code)]
pub struct ModuleInfo {
    name: String,
    addr: usize,
    size: usize,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct FunctionInfo {
    pub name: String,
    pub addr: usize,
}

pub struct Modules {
    pub process: Option<winapi::um::winnt::HANDLE>,
    modules_intervals: Vec<Iv>,
    modules: Vec<ModuleInfo>,
    modules_tree: Lapper<usize, usize>,
    functions: Vec<FunctionInfo>,
    functions_intervals: Vec<Iv>,
    opcodes: Vec<(usize, Vec<Instruction>)>,
}

impl Modules {
    fn update(&mut self) {
        self.opcodes.sort_by(|l, r| l.0.cmp(&r.0));
        self.functions.sort_by(|l, r| l.addr.cmp(&r.addr));
        self.modules_tree = Lapper::new(self.modules_intervals.clone());
    }

    #[allow(dead_code)]
    pub fn load_com(&mut self, _: &str, _: usize) {
        // if api == "d3d9" {
        //     let mut newfunctions = vec![];

        //     let vtable = crate::com::get_d3d9_vtable(addr);
        //     for (addr, name) in vtable {
        //         newfunctions.push((name.to_string(), addr as usize));
        //     }

        //     self.add_newfunctions(newfunctions);
        // } else {
        //     panic!();
        // }
        todo!();
    }

    fn add_newfunctions(&mut self, mut newfunctions: Vec<(String, usize)>) {
        if newfunctions.len() == 0 {
            return;
        }
        newfunctions.sort_by(|l, r| l.1.cmp(&r.1));
        let mut ranges: Vec<_> = newfunctions
            .iter()
            .zip(newfunctions.iter().skip(1))
            .map(|(l, r)| (l.0.clone(), l.1, r.1 - l.1))
            .collect();
        let l = newfunctions.last().unwrap();
        ranges.push((l.0.clone(), l.1, 100));
        for (name, start, size) in ranges.iter() {
            let _ = start + size;

            let size = if *size > 1000000 {
                debug!(target:"modules", "Ignoring function {} because of size", name);
                10
            } else {
                *size
            };
            if let Ok(bytes) = read_process_memory(self.process.unwrap(), *start, size) {
                let mut instructions = vec![];

                let mut decoder =
                    iced_x86::Decoder::new(32, bytes.as_slice(), iced_x86::DecoderOptions::NONE);
                while decoder.can_decode() {
                    let mut instruction = iced_x86::Instruction::default();
                    decoder.decode_out(&mut instruction);
                    instructions.push(instruction);
                }

                self.opcodes.push((*start, instructions));
            }

            self.functions.push(FunctionInfo {
                name: name.clone(),
                addr: *start,
            });
            self.functions_intervals.push(Iv {
                start: *start,
                stop: start + size,
                val: self.functions.len(),
            });
        }

        if let Some(_) = newfunctions.last() {}

        self.update();
    }

    pub fn load_module(&mut self, base_addr: usize, size: usize, name: &str) -> Result<(), u32> {
        let process = self.process.unwrap();

        let dosheader = parse_at::<exe::headers::ImageDOSHeader>(base_addr, process)?;
        let _ = parse_at::<u32>(base_addr + (dosheader.e_lfanew.0 as usize), process)?;
        let fileheader = parse_at::<exe::headers::ImageFileHeader>(
            base_addr + (dosheader.e_lfanew.0 as usize) + 4,
            process,
        )?;

        const IMAGE_DIRECTORY_ENTRY_EXPORT: usize = 0;
        // 32 bits
        let (export_datadir, export_dir) = if fileheader.machine == 0x014c {
            let _ = parse_at::<exe::headers::ImageOptionalHeader32>(
                base_addr
                    + (dosheader.e_lfanew.0 as usize)
                    + 4
                    + std::mem::size_of::<exe::headers::ImageFileHeader>(),
                process,
            )?;
            let optional_header32_data = parse_at::<[exe::headers::ImageDataDirectory; 16]>(
                base_addr
                    + (dosheader.e_lfanew.0 as usize)
                    + 4
                    + std::mem::size_of::<exe::headers::ImageFileHeader>()
                    + std::mem::size_of::<exe::headers::ImageOptionalHeader32>(),
                process,
            )?;

            let exportva = optional_header32_data[IMAGE_DIRECTORY_ENTRY_EXPORT].virtual_address;
            (
                optional_header32_data[IMAGE_DIRECTORY_ENTRY_EXPORT].clone(),
                parse_at::<exe::headers::ImageExportDirectory>(
                    base_addr + exportva.0 as usize,
                    process,
                )?,
            )
        }
        //64 bits if fileheader.machine == 0x8664
        else {
            let _ = parse_at::<exe::headers::ImageOptionalHeader64>(
                base_addr
                    + (dosheader.e_lfanew.0 as usize)
                    + 4
                    + std::mem::size_of::<exe::headers::ImageFileHeader>(),
                process,
            )?;
            let optional_header64_data = parse_at::<[exe::headers::ImageDataDirectory; 16]>(
                base_addr
                    + (dosheader.e_lfanew.0 as usize)
                    + 4
                    + std::mem::size_of::<exe::headers::ImageFileHeader>()
                    + std::mem::size_of::<exe::headers::ImageOptionalHeader64>(),
                process,
            )?;

            let exportva = optional_header64_data[IMAGE_DIRECTORY_ENTRY_EXPORT].virtual_address;
            (
                optional_header64_data[IMAGE_DIRECTORY_ENTRY_EXPORT].clone(),
                parse_at::<exe::headers::ImageExportDirectory>(
                    base_addr + exportva.0 as usize,
                    process,
                )?,
            )
        };

        let mut newfunctions = vec![];

        if export_dir.address_of_functions.0 != 0 {
            let functions = parse_at_n::<u32>(
                base_addr + export_dir.address_of_functions.0 as usize,
                process,
                export_dir.number_of_functions as usize,
            )?;
            let names = parse_at_n::<u32>(
                base_addr + export_dir.address_of_names.0 as usize,
                process,
                export_dir.number_of_names as usize,
            )?;
            let idxs = parse_at_n::<u16>(
                base_addr + export_dir.address_of_name_ordinals.0 as usize,
                process,
                export_dir.number_of_names as usize,
            )?;

            debug!(target:"modules", "Functions: {} Names: {}", functions.len(), names.len());

            for i in 0..export_dir.number_of_names {
                let nameaddr = base_addr + names[i as usize] as usize;
                let name = read_string_char_by_char(process, nameaddr).unwrap();

                let fid = idxs[i as usize] as usize;
                let addr = functions[fid] as usize;

                if (addr >= export_datadir.virtual_address.0 as usize)
                    && (addr <= (export_datadir.virtual_address.0 + export_datadir.size) as usize)
                {
                    let _ = read_string_char_by_char(process, base_addr + addr);
                    // println!(
                    //     "\t\t[{:?}] aka [{}] @ 0x{:X?}",
                    //     exported_name,
                    //     name,
                    //     base_addr + addr
                    // );
                } else {
                    newfunctions.push((name, base_addr + addr));
                }
            }
        } else if size < 10000000 {
            //No exported function. Let us scan the code to function prologues and calls.
            let mem = read_process_memory(process, base_addr, size).unwrap();

            for addr in 0..size {
                if (mem[addr + 0] == 0x55 && mem[addr + 1] == 0x89)
                || ((addr > 2) && (mem[addr - 2] == 0xcc && mem[addr - 1] == 0xcc))
                {
                    let name = format!("f_{:X?}", base_addr + addr);
                    newfunctions.push((name, base_addr + addr));
                }

            }
        }

        self.add_newfunctions(newfunctions);

        self.modules_intervals.push(Iv {
            start: base_addr as usize,
            stop: base_addr + size,
            val: self.modules.len(),
        });
        self.modules.push(ModuleInfo {
            name: name.to_string(),
            addr: base_addr,
            size: size,
        });

        self.update();

        Ok(())
    }

    #[allow(dead_code)]
    pub fn build_modules_tree(&mut self) {
        unsafe {
            let process = self.process.unwrap();

            let pid = winapi::um::processthreadsapi::GetProcessId(process);

            let s = winapi::um::tlhelp32::CreateToolhelp32Snapshot(
                winapi::um::tlhelp32::TH32CS_SNAPALL,
                pid,
            );

            let mut m: winapi::um::tlhelp32::MODULEENTRY32 = Default::default();
            m.dwSize = std::mem::size_of::<winapi::um::tlhelp32::MODULEENTRY32>() as u32;

            winapi::um::tlhelp32::Module32First(s, &mut m);

            loop {
                let name = string_from_array_with_zero(&m.szModule[..]);
                let binpath = string_from_array_with_zero(&m.szExePath[..]);
                debug!(target:"modules", "\tModule: {:?} {:?}", name, binpath);
                // println!("{:?}", );
                // println!("{:?}", m.th32ProcessID);
                // println!("{:?}", m.GlblcntUsage);
                // println!("{:?}", m.ProccntUsage);
                // println!("{:?}", m.modBaseAddr);
                // println!("{:?}", m.modBaseSize);

                let _ = self.load_module(
                    m.modBaseAddr as usize,
                    m.modBaseSize as usize,
                    name.as_str(),
                );

                if winapi::um::tlhelp32::Module32Next(s, &mut m) == 0 {
                    break;
                }
            }

            self.update();
        }
    }

    pub fn new() -> Self {
        Self {
            process: None,
            modules: vec![],
            modules_tree: Lapper::new(vec![]),
            functions: vec![],
            modules_intervals: vec![],
            functions_intervals: vec![],
            opcodes: vec![],
        }
    }

    #[allow(dead_code)]
    fn get_name(m: &winapi::um::tlhelp32::MODULEENTRY32) -> String {
        string_from_array_with_zero(&m.szModule[..])
    }

    #[allow(dead_code)]
    fn get_path(m: &winapi::um::tlhelp32::MODULEENTRY32) -> String {
        string_from_array_with_zero(&m.szExePath[..])
    }

    #[allow(dead_code)]
    pub fn get_module_at(&self, addr: usize) -> Option<&ModuleInfo> {
        let index = match self.modules.binary_search_by(|x| x.addr.cmp(&addr)) {
            Ok(index) => index as isize,
            Err(index) => index as isize - 1,
        };
        if index > 0 {
            self.modules.get(index as usize)
        } else {
            None
        }
    }

    pub fn get_function_at(&self, addr: usize) -> Option<&FunctionInfo> {
        let index = match self.functions.binary_search_by(|x| x.addr.cmp(&addr)) {
            Ok(index) => index as isize,
            Err(index) => index as isize - 1,
        };
        if index > 0 {
            self.functions.get(index as usize)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn get_instructions_at(&self, addr: usize) -> Option<&(usize, Vec<Instruction>)> {
        let index = match self.opcodes.binary_search_by(|x| x.0.cmp(&addr)) {
            Ok(index) => index as isize,
            Err(index) => index as isize - 1,
        };
        if index > 0 {
            self.opcodes.get(index as usize)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn get_instruction_at(&self, addr: usize) -> Option<(usize, &Instruction)> {
        let index = match self.opcodes.binary_search_by(|x| x.0.cmp(&addr)) {
            Ok(index) => index as isize,
            Err(index) => index as isize - 1,
        };
        if index > 0 {
            let (mut eip, instructions) = self.opcodes.get(index as usize).unwrap();
            let mut idx = 0;
            loop {
                if let Some(i) = instructions.get(idx) {
                    if eip >= addr {
                        break Some((eip, i));
                    }
                    eip += i.len();
                    idx += 1;
                } else {
                    break None;
                }
            }
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn get_next_instruction_after(&self, addr: usize) -> Option<(usize, &Instruction)> {
        let index = match self.opcodes.binary_search_by(|x| x.0.cmp(&addr)) {
            Ok(index) => index as isize,
            Err(index) => index as isize - 1,
        };
        if index > 0 {
            let (mut eip, instructions) = self.opcodes.get(index as usize).unwrap();
            let mut idx = 0;
            loop {
                let i = &instructions[idx];
                if eip > addr {
                    break Some((eip, i));
                }

                eip += i.len();
                idx += 1;
            }
        } else {
            None
        }
    }

    pub fn get_function_addr<S1: AsRef<str>, S2: AsRef<str>>(
        &self,
        _: S1,
        symbol: S2,
    ) -> Option<usize> {
        let function = symbol.as_ref();
        self.functions
            .iter()
            .find(|x| x.name == function)
            .map(|x| x.addr)
    }
}

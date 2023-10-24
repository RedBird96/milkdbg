use bitflags::*;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ForwardOffset(pub u32);

impl Into<usize> for ForwardOffset {
    fn into(self) -> usize {
        self.0 as usize
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RVA(pub u32);

impl RVA {
    pub fn to_va(&self, base: usize) -> usize {
        base + self.0 as usize
    }
}

impl _core::fmt::Debug for RVA {
    fn fmt(&self, f: &mut _core::fmt::Formatter<'_>) -> _core::fmt::Result {
        f.write_fmt(format_args!("0x{:X?}", self.0))
    }
}

impl std::ops::Add<usize> for RVA {
    type Output = usize;

    fn add(self, rhs: usize) -> Self::Output {
        self.0 as usize + rhs
    }
}

impl std::ops::Add<RVA> for usize {
    type Output = usize;

    fn add(self, rhs: RVA) -> Self::Output {
        self + rhs.0 as usize
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PeDosHeader {
    pub e_magic: u16,
    pub e_cblp: u16,
    pub e_cp: u16,
    pub e_crlc: u16,
    pub e_cparhdr: u16,
    pub e_minalloc: u16,
    pub e_maxalloc: u16,
    pub e_ss: u16,
    pub e_sp: u16,
    pub e_csum: u16,
    pub e_ip: u16,
    pub e_cs: u16,
    pub e_lfarlc: u16,
    pub e_ovno: u16,
    pub e_res: [u16; 4],
    pub e_oemid: u16,
    pub e_oeminfo: u16,
    pub e_res2: [u16; 10],
    pub e_lfanew: ForwardOffset,
}

bitflags! {
    /// A bitflag structure representing file characteristics in the file header.
    pub struct FileCharacteristics: u16 {
        const RELOCS_STRIPPED         = 0x0001;
        const EXECUTABLE_IMAGE        = 0x0002;
        const LINE_NUMS_STRIPPED      = 0x0004;
        const LOCAL_SYMS_STRIPPED     = 0x0008;
        const AGGRESSIVE_WS_TRIM      = 0x0010;
        const LARGE_ADDRESS_AWARE     = 0x0020;
        const BYTES_REVERSED_LO       = 0x0080;
        const MACHINE_32BIT           = 0x0100;
        const DEBUG_STRIPPED          = 0x0200;
        const REMOVABLE_RUN_FROM_SWAP = 0x0400;
        const NET_RUN_FROM_SWAP       = 0x0800;
        const SYSTEM                  = 0x1000;
        const DLL                     = 0x2000;
        const UP_SYSTEM_ONLY          = 0x4000;
        const BYTES_REVERSED_HI       = 0x8000;
    }
}

#[repr(u16)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ImageFileMachine {
    Unknown = 0x0000,
    TargetHost = 0x0001,
    I386 = 0x014C,
    R3000 = 0x0162,
    R4000 = 0x0166,
    R10000 = 0x0168,
    WCEMIPSV2 = 0x0169,
    Alpha = 0x0184,
    SH3 = 0x01A2,
    SH3DSP = 0x01A3,
    SH3E = 0x01A4,
    SH4 = 0x01A6,
    SH5 = 0x01A8,
    ARM = 0x01C0,
    Thumb = 0x01C2,
    ARMNT = 0x01C4,
    AM33 = 0x01D3,
    PowerPC = 0x01F0,
    PowerPCFP = 0x01F1,
    IA64 = 0x0200,
    MIPS16 = 0x0266,
    Alpha64 = 0x0284,
    MIPSFPU = 0x0366,
    MIPSFPU16 = 0x0466,
    TRICORE = 0x0520,
    CEF = 0x0CEF,
    EBC = 0x0EBC,
    AMD64 = 0x8664,
    M32R = 0x9041,
    ARM64 = 0xAA64,
    CEE = 0xC0EE,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PeCoffHeader {
    pub machine: ImageFileMachine,
    pub number_of_sections: u16,
    pub time_date_stamp: u32,
    pub pointer_to_symbol_table: ForwardOffset,
    pub number_of_symbols: u32,
    pub size_of_optional_header: u16,
    pub characteristics: FileCharacteristics,
}

bitflags! {
    /// A series of bitflags representing DLL characteristics.
    pub struct DLLCharacteristics: u16 {
        const RESERVED1             = 0x0001;
        const RESERVED2             = 0x0002;
        const RESERVED4             = 0x0004;
        const RESERVED8             = 0x0008;
        const HIGH_ENTROPY_VA       = 0x0020;
        const DYNAMIC_BASE          = 0x0040;
        const FORCE_INTEGRITY       = 0x0080;
        const NX_COMPAT             = 0x0100;
        const NO_ISOLATION          = 0x0200;
        const NO_SEH                = 0x0400;
        const NO_BIND               = 0x0800;
        const APPCONTAINER          = 0x1000;
        const WDM_DRIVER            = 0x2000;
        const GUARD_CF              = 0x4000;
        const TERMINAL_SERVER_AWARE = 0x8000;
    }
}

#[repr(u16)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum OptionalHeader32Magic {
    PE32 = 0x10b,
    PE32Plus = 0x20b,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PeOptionalHeader32 {
    pub magic: OptionalHeader32Magic,
    pub major_linker_version: u8,
    pub minor_linker_version: u8,
    pub size_of_code: u32,
    pub size_of_initialized_data: u32,
    pub size_of_uninitialized_data: u32,
    pub address_of_entry_point: RVA,
    pub base_of_code: RVA,
    pub base_of_data: RVA,
    pub image_base: u32,
    pub section_alignment: u32,
    pub file_alignment: u32,
    pub major_operating_system_version: u16,
    pub minor_operating_system_version: u16,
    pub major_image_version: u16,
    pub minor_image_version: u16,
    pub major_subsystem_version: u16,
    pub minor_subsystem_version: u16,
    pub win32_version_value: u32,
    pub size_of_image: u32,
    pub size_of_headers: u32,
    pub checksum: u32,
    pub subsystem: u16,
    pub dll_characteristics: DLLCharacteristics,
    pub size_of_stack_reserve: u32,
    pub size_of_stack_commit: u32,
    pub size_of_heap_reserve: u32,
    pub size_of_heap_commit: u32,
    pub loader_flags: u32,
    pub number_of_rva_and_sizes: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PeDataDirectory {
    pub virtual_address: RVA,
    pub size: u32,
}

bitflags! {
    /// A series of bitflags representing section characteristics.
    pub struct SectionCharacteristics: u32 {
        const TYPE_REG               = 0x00000000;
        const TYPE_DSECT             = 0x00000001;
        const TYPE_NOLOAD            = 0x00000002;
        const TYPE_GROUP             = 0x00000004;
        const TYPE_NO_PAD            = 0x00000008;
        const TYPE_COPY              = 0x00000010;
        const CNT_CODE               = 0x00000020;
        const CNT_INITIALIZED_DATA   = 0x00000040;
        const CNT_UNINITIALIZED_DATA = 0x00000080;
        const LNK_OTHER              = 0x00000100;
        const LNK_INFO               = 0x00000200;
        const TYPE_OVER              = 0x00000400;
        const LNK_REMOVE             = 0x00000800;
        const LNK_COMDAT             = 0x00001000;
        const RESERVED               = 0x00002000;
        const MEM_PROTECTED          = 0x00004000;
        const NO_DEFER_SPEC_EXC      = 0x00004000;
        const GPREL                  = 0x00008000;
        const MEM_FARDATA            = 0x00008000;
        const MEM_SYSHEAP            = 0x00010000;
        const MEM_PURGEABLE          = 0x00020000;
        const MEM_16BIT              = 0x00020000;
        const MEM_LOCKED             = 0x00040000;
        const MEM_PRELOAD            = 0x00080000;
        const ALIGN_1BYTES           = 0x00100000;
        const ALIGN_2BYTES           = 0x00200000;
        const ALIGN_4BYTES           = 0x00300000;
        const ALIGN_8BYTES           = 0x00400000;
        const ALIGN_16BYTES          = 0x00500000;
        const ALIGN_32BYTES          = 0x00600000;
        const ALIGN_64BYTES          = 0x00700000;
        const ALIGN_128BYTES         = 0x00800000;
        const ALIGN_256BYTES         = 0x00900000;
        const ALIGN_512BYTES         = 0x00A00000;
        const ALIGN_1024BYTES        = 0x00B00000;
        const ALIGN_2048BYTES        = 0x00C00000;
        const ALIGN_4096BYTES        = 0x00D00000;
        const ALIGN_8192BYTES        = 0x00E00000;
        const ALIGN_MASK             = 0x00F00000;
        const LNK_NRELOC_OVFL        = 0x01000000;
        const MEM_DISCARDABLE        = 0x02000000;
        const MEM_NOT_CACHED         = 0x04000000;
        const MEM_NOT_PAGED          = 0x08000000;
        const MEM_SHARED             = 0x10000000;
        const MEM_EXECUTE            = 0x20000000;
        const MEM_READ               = 0x40000000;
        const MEM_WRITE              = 0x80000000;
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PeExportSection {
    pub characteristics: SectionCharacteristics,
    pub time_date_stamp: u32,
    pub major_version: u16,
    pub minor_version: u16,
    pub name: RVA,
    pub base: u32,
    pub number_of_functions: u32,
    pub number_of_names: u32,
    pub address_of_functions: RVA,
    pub address_of_names: RVA,
    pub address_of_name_ordinals: RVA,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PeImportBinary {
    pub original_first_thunk: RVA,
    pub time_date_stamp: u32,
    pub forwarder_chain: u32,
    pub name: RVA,
    pub first_thunk: RVA,
}

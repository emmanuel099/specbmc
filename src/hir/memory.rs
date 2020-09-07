use bitflags::bitflags;

bitflags! {
    pub struct MemoryPermissions: u32 {
        const READ    = 0b001;
        const WRITE   = 0b010;
        const EXECUTE = 0b100;
    }
}

/// A (half-open) range bounded inclusively below and exclusively above (start_address..end_address).
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct MemorySection {
    start_address: u64,
    end_address: u64,
    permissions: MemoryPermissions,
}

impl MemorySection {
    pub fn new(start_address: u64, end_address: u64, permissions: MemoryPermissions) -> Self {
        Self {
            start_address,
            end_address,
            permissions,
        }
    }

    pub fn start_address(&self) -> u64 {
        self.start_address
    }

    pub fn end_address(&self) -> u64 {
        self.end_address
    }

    pub fn addresses(&self) -> impl Iterator<Item = u64> {
        self.start_address..self.end_address
    }

    pub fn permissions(&self) -> MemoryPermissions {
        self.permissions
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Default)]
pub struct Memory {
    sections: Vec<MemorySection>,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            sections: Vec::default(),
        }
    }

    pub fn insert_section(&mut self, section: MemorySection) {
        self.sections.push(section)
    }

    pub fn sections(&self) -> &[MemorySection] {
        &self.sections
    }
}

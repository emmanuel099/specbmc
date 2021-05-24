use crate::error::Result;
use crate::expr::{Expression, Sort, Variable};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Memory {
    Store(usize),
    Load(usize),
}

impl fmt::Display for Memory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store(bit_width) => write!(f, "(store {})", bit_width),
            Self::Load(bit_width) => write!(f, "(load {})", bit_width),
        }
    }
}

impl Memory {
    pub fn variable() -> Variable {
        Variable::new("_memory", Sort::memory())
    }

    pub fn load(bit_width: usize, memory: Expression, addr: Expression) -> Result<Expression> {
        memory.sort().expect_memory()?;
        addr.sort().expect_word()?;

        Ok(Expression::new(
            Self::Load(bit_width).into(),
            vec![memory, addr],
            Sort::BitVector(bit_width),
        ))
    }

    pub fn store(memory: Expression, addr: Expression, value: Expression) -> Result<Expression> {
        memory.sort().expect_memory()?;
        addr.sort().expect_word()?;
        value.sort().expect_bit_vector()?;

        let bit_width = value.sort().unwrap_bit_vector();

        let result_sort = memory.sort().clone();
        Ok(Expression::new(
            Self::Store(bit_width).into(),
            vec![memory, addr, value],
            result_sort,
        ))
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct MemoryValue {
    content: BTreeMap<u64, u64>,
    default_byte: u8,
    default_word: u64,
}

impl MemoryValue {
    pub fn new(default_byte: u8) -> Self {
        Self {
            content: BTreeMap::new(),
            default_byte,
            default_word: Self::fill_word(default_byte),
        }
    }

    pub fn content(&self) -> &BTreeMap<u64, u64> {
        &self.content
    }

    pub fn default_word(&self) -> u64 {
        self.default_word
    }

    pub fn load(&self, addr: u64) -> u8 {
        let (row, offset) = Self::address_to_row_and_offset(addr);
        self.content
            .get(&row)
            .map(|word| (word >> offset) as u8)
            .unwrap_or(self.default_byte)
    }

    pub fn store(&mut self, addr: u64, value: u8) {
        let (row, offset) = Self::address_to_row_and_offset(addr);
        let word = self.content.entry(row).or_insert(self.default_word);
        *word &= !(0xFF_u64 << offset);
        *word |= (value as u64) << offset;
    }

    fn fill_word(byte: u8) -> u64 {
        let mut word: u64 = 0;
        for _ in 0..8 {
            word = (word << 8) | (byte as u64);
        }
        word
    }

    fn address_to_row_and_offset(addr: u64) -> (u64, u8) {
        let mask: u64 = 0x07;
        let row = addr & !mask;
        let byte = (addr & mask) as u8;
        let offset = (7 - byte) * 8;
        (row, offset)
    }
}

impl fmt::Display for MemoryValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        let mut is_first = true;
        for (address, word) in self.content.iter() {
            if !is_first {
                write!(f, ", ")?;
            }
            write!(f, "0x{:X}: 0x{:016X}", address, word)?;
            is_first = false;
        }
        write!(f, ", …: 0x{:016X}]", self.default_word)
    }
}
